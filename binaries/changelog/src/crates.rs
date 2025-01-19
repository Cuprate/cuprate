//! TODO

use std::process::Command;

use serde::{Deserialize, Serialize};

/// [`CargoMetadata::packages`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Package {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct CuprateCrates {
    pub packages: Vec<Package>,
}

impl CuprateCrates {
    pub fn new() -> Self {
        let output = Command::new("cargo")
            .args(["metadata", "--no-deps"])
            .output()
            .unwrap()
            .stdout;

        serde_json::from_slice(&output).unwrap()
    }

    pub fn crate_version(&self, crate_name: &str) -> &str {
        &self
            .packages
            .iter()
            .find(|p| p.name == crate_name)
            .unwrap()
            .version
    }
}
