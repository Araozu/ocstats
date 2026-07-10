use std::{
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
        let path = env::var_os("OCSTATS_PRICING_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("pricing.yaml"));
        Self::load(path)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let contents = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_valid() {
        let catalog: PricingCatalog =
            serde_yaml::from_str(include_str!("../pricing.yaml")).unwrap();
        assert_eq!(catalog.models.len(), 15);
        assert!(catalog.models.iter().any(|model| model.slug == "gpt-5.5"));
    }
}
