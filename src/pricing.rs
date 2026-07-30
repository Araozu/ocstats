use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use chrono::DateTime;
use serde::{Deserialize, Serialize, de, de::Error as DeError, ser::SerializeStruct};

use crate::Error;

const LEGACY_EFFECTIVE_FROM: &str = "1970-01-01T00:00:00Z";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PricePeriod {
    pub effective_from: String,
    pub input: f64,
    pub cached_write: Option<f64>,
    pub cached_read: Option<f64>,
    pub output: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    pub provider: String,
    pub slug: String,
    pub prices: Vec<PricePeriod>,
}

impl Serialize for ModelPricing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let current = self.prices.last();
        let mut model = serializer.serialize_struct("ModelPricing", 7)?;
        model.serialize_field("provider", &self.provider)?;
        model.serialize_field("slug", &self.slug)?;
        model.serialize_field("input", &current.map(|price| price.input))?;
        model.serialize_field(
            "cached_write",
            &current.and_then(|price| price.cached_write),
        )?;
        model.serialize_field("cached_read", &current.and_then(|price| price.cached_read))?;
        model.serialize_field("output", &current.map(|price| price.output))?;
        model.serialize_field("prices", &self.prices)?;
        model.end()
    }
}

#[derive(Debug, Deserialize)]
struct RawModelPricing {
    provider: String,
    slug: String,
    #[serde(default)]
    prices: Option<Vec<PricePeriod>>,
    #[serde(default)]
    input: Option<f64>,
    #[serde(default)]
    cached_write: Option<f64>,
    #[serde(default)]
    cached_read: Option<f64>,
    #[serde(default)]
    output: Option<f64>,
}

impl<'de> Deserialize<'de> for ModelPricing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let raw = RawModelPricing::deserialize(deserializer)?;
        let has_legacy_rates = raw.input.is_some()
            || raw.cached_write.is_some()
            || raw.cached_read.is_some()
            || raw.output.is_some();
        let prices = match (raw.prices, has_legacy_rates) {
            (Some(prices), _) => prices,
            (None, false) => {
                return Err(D::Error::custom(
                    "a model must define a prices list or legacy top-level rates",
                ));
            }
            (None, true) => vec![PricePeriod {
                effective_from: LEGACY_EFFECTIVE_FROM.to_owned(),
                input: raw
                    .input
                    .ok_or_else(|| D::Error::custom("legacy model is missing input rate"))?,
                cached_write: raw.cached_write,
                cached_read: raw.cached_read,
                output: raw
                    .output
                    .ok_or_else(|| D::Error::custom("legacy model is missing output rate"))?,
            }],
        };

        Ok(Self {
            provider: raw.provider,
            slug: raw.slug,
            prices,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PricingCatalog {
    pub models: Vec<ModelPricing>,
}

impl PricingCatalog {
    pub fn load_default() -> Result<Self, Error> {
        match env::var_os("OCSTATS_PRICING_FILE") {
            Some(path) => Self::load(PathBuf::from(path)),
            None => Self::from_yaml(include_str!("../pricing.yaml")),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let contents = fs::read_to_string(path)?;
        Self::from_yaml(&contents)
    }

    fn from_yaml(contents: &str) -> Result<Self, Error> {
        let catalog: Self = serde_yaml::from_str(contents)?;
        catalog.validate()?;
        Ok(catalog)
    }

    fn validate(&self) -> Result<(), Error> {
        let mut model_keys = BTreeSet::new();
        for model in &self.models {
            if model.provider.trim().is_empty() || model.slug.trim().is_empty() {
                return Err(Error::PricingValidation(
                    "provider and slug must not be empty".into(),
                ));
            }
            if !model_keys.insert((model.provider.as_str(), model.slug.as_str())) {
                return Err(Error::PricingValidation(format!(
                    "duplicate model {}:{}",
                    model.provider, model.slug
                )));
            }
            if model.prices.is_empty() {
                return Err(Error::PricingValidation(format!(
                    "model {}:{} has no price periods",
                    model.provider, model.slug
                )));
            }

            let mut previous = None;
            for price in &model.prices {
                let effective_from =
                    DateTime::parse_from_rfc3339(&price.effective_from).map_err(|error| {
                        Error::PricingValidation(format!(
                            "model {}:{} has invalid effective_from {}: {error}",
                            model.provider, model.slug, price.effective_from
                        ))
                    })?;
                if effective_from.offset().local_minus_utc() != 0 {
                    return Err(Error::PricingValidation(format!(
                        "model {}:{} effective_from must use UTC",
                        model.provider, model.slug
                    )));
                }
                if effective_from.timestamp_subsec_nanos() % 1_000_000 != 0 {
                    return Err(Error::PricingValidation(format!(
                        "model {}:{} effective_from must be representable in milliseconds",
                        model.provider, model.slug
                    )));
                }
                if previous.is_some_and(|previous| effective_from <= previous) {
                    return Err(Error::PricingValidation(format!(
                        "model {}:{} price periods must be strictly ordered",
                        model.provider, model.slug
                    )));
                }
                previous = Some(effective_from);
                validate_rate(model, "input", price.input)?;
                validate_rate(model, "output", price.output)?;
                if let Some(rate) = price.cached_write {
                    validate_rate(model, "cached_write", rate)?;
                }
                if let Some(rate) = price.cached_read {
                    validate_rate(model, "cached_read", rate)?;
                }
            }
        }
        Ok(())
    }
}

fn validate_rate(model: &ModelPricing, name: &str, rate: f64) -> Result<(), Error> {
    if !rate.is_finite() || rate < 0.0 {
        return Err(Error::PricingValidation(format!(
            "model {}:{} has invalid {name} rate {rate}",
            model.provider, model.slug
        )));
    }
    Ok(())
}

#[derive(Debug)]
pub struct PricingRequests {
    path: PathBuf,
}

impl PricingRequests {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn record(&self, slug: &str) -> Result<(), Error> {
        let existing = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };

        let mut slugs = BTreeSet::new();
        slugs.extend(
            existing
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
        slugs.insert(slug.to_owned());

        let mut contents = slugs.into_iter().collect::<Vec<_>>().join("\n");
        contents.push('\n');
        fs::write(&self.path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_valid() {
        let catalog = PricingCatalog::load_default().unwrap();
        assert_eq!(catalog.models.len(), 30);
        assert!(catalog.models.iter().any(|model| model.slug == "gpt-5.5"));
        assert!(catalog.models[0].prices[0].effective_from.ends_with('Z'));
    }

    #[test]
    fn legacy_catalog_entries_are_normalized() {
        let catalog: PricingCatalog = serde_yaml::from_str(
            "models:\n  - provider: test\n    slug: model\n    input: 1\n    cached_write: null\n    cached_read: 0.1\n    output: 2\n",
        )
        .unwrap();
        assert_eq!(catalog.models[0].prices.len(), 1);
        assert_eq!(
            catalog.models[0].prices[0].effective_from,
            LEGACY_EFFECTIVE_FROM
        );
    }

    #[test]
    fn serialized_catalogs_round_trip_with_legacy_rates() {
        let catalog = PricingCatalog {
            models: vec![ModelPricing {
                provider: "test".into(),
                slug: "model".into(),
                prices: vec![
                    PricePeriod {
                        effective_from: "2026-01-01T00:00:00Z".into(),
                        input: 1.0,
                        cached_write: None,
                        cached_read: Some(0.1),
                        output: 2.0,
                    },
                    PricePeriod {
                        effective_from: "2026-07-01T00:00:00Z".into(),
                        input: 3.0,
                        cached_write: None,
                        cached_read: Some(0.3),
                        output: 4.0,
                    },
                ],
            }],
        };
        let serialized = serde_yaml::to_string(&catalog).unwrap();
        let parsed: PricingCatalog = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(parsed, catalog);

        let legacy: PricingCatalog = serde_yaml::from_str(
            "models:\n  - provider: test\n    slug: model\n    input: 1\n    cached_write: null\n    cached_read: 0.1\n    output: 2\n",
        )
        .unwrap();
        let legacy_serialized = serde_yaml::to_string(&legacy).unwrap();
        assert_eq!(
            serde_yaml::from_str::<PricingCatalog>(&legacy_serialized).unwrap(),
            legacy
        );
    }

    #[test]
    fn invalid_price_periods_are_rejected() {
        let catalog = PricingCatalog {
            models: vec![ModelPricing {
                provider: "test".into(),
                slug: "model".into(),
                prices: vec![
                    PricePeriod {
                        effective_from: "2026-01-01T00:00:00Z".into(),
                        input: 1.0,
                        cached_write: None,
                        cached_read: None,
                        output: 2.0,
                    },
                    PricePeriod {
                        effective_from: "2026-01-01T00:00:00Z".into(),
                        input: 1.0,
                        cached_write: None,
                        cached_read: None,
                        output: 2.0,
                    },
                ],
            }],
        };
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn non_utc_price_periods_are_rejected() {
        let catalog = PricingCatalog {
            models: vec![ModelPricing {
                provider: "test".into(),
                slug: "model".into(),
                prices: vec![PricePeriod {
                    effective_from: "2026-01-01T01:00:00+01:00".into(),
                    input: 1.0,
                    cached_write: None,
                    cached_read: None,
                    output: 2.0,
                }],
            }],
        };
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn sub_millisecond_price_periods_are_rejected() {
        let catalog = PricingCatalog {
            models: vec![ModelPricing {
                provider: "test".into(),
                slug: "model".into(),
                prices: vec![PricePeriod {
                    effective_from: "2026-01-01T00:00:00.000001Z".into(),
                    input: 1.0,
                    cached_write: None,
                    cached_read: None,
                    output: 2.0,
                }],
            }],
        };
        assert!(catalog.validate().is_err());
    }
}
