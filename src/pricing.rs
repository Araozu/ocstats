use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPricing {
    pub provider: String,
    pub slug: String,
    pub input: f64,
    pub cached_write: Option<f64>,
    pub cached_read: Option<f64>,
    pub output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PricingCatalog {
    pub models: Vec<ModelPricing>,
}

impl PricingCatalog {
    pub fn load_default() -> Result<Self, Error> {
        match env::var_os("OCSTATS_PRICING_FILE") {
            Some(path) => Self::load(PathBuf::from(path)),
            None => Ok(serde_yaml::from_str(include_str!("../pricing.yaml"))?),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let contents = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
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
        let catalog: PricingCatalog =
            serde_yaml::from_str(include_str!("../pricing.yaml")).unwrap();
        assert_eq!(catalog.models.len(), 30);
        assert!(catalog.models.iter().any(|model| model.slug == "gpt-5.5"));
    }
}
