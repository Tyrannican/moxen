use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::addon::Addon;

// TODO: Custom error handling

const VERSIONS: [GameVersion; 4] = [
    GameVersion::Retail,
    GameVersion::Beta,
    GameVersion::Ptr,
    GameVersion::Classic,
];

#[derive(Deserialize, Serialize, Default, Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum GameVersion {
    #[default]
    Retail,
    Beta,
    Ptr,
    Classic,
}

impl GameVersion {
    pub fn registry_path(&self) -> Result<PathBuf> {
        let registry = load_path("registry").context("loading registry path")?;
        if !registry.exists() {
            std::fs::create_dir_all(&registry).context("creating registry dir")?;
        }

        Ok(registry.join(format!("{}.json", self)))
    }

    pub fn suffix(&self) -> String {
        match self {
            Self::Retail => "_retail_".to_string(),
            Self::Beta => "_beta_".to_string(),
            Self::Ptr => "_ptr_".to_string(),
            Self::Classic => "_classic_".to_string(),
        }
    }
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retail => write!(f, "retail"),
            Self::Beta => write!(f, "beta"),
            Self::Classic => write!(f, "classic"),
            Self::Ptr => write!(f, "ptr"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct MoxenConfig {
    pub api_key: String,
    pub version: GameVersion,
}

impl MoxenConfig {
    pub fn is_initialised() -> Result<bool> {
        let cfg_path = load_path("config.toml").context("config file path")?;
        let registries_exist = VERSIONS.iter().all(|version| {
            let path = version
                .registry_path()
                .context("loading registry path")
                .expect("error loading path");
            path.exists()
        });

        Ok(cfg_path.exists() && registries_exist)
    }

    pub fn initialise() -> Result<()> {
        let cfg_path = load_path("config.toml")?;
        let api_key =
            rpassword::prompt_password("Enter Curseforge API Key: ").context("reading password")?;

        let cfg = MoxenConfig {
            api_key,
            version: GameVersion::default(),
        };

        let content = toml::to_string_pretty(&cfg).context("serialising config")?;
        std::fs::write(&cfg_path, &content).context("writing out config file")?;

        Ok(())
    }

    pub fn load() -> Result<Self> {
        let cfg_path = load_path("config.toml")?;
        let content = std::fs::read_to_string(&cfg_path).context("reading config file")?;
        toml::from_str(&content).context("deserialising config")
    }

    pub fn save(&self) -> Result<()> {
        let cfg_path = load_path("config.toml")?;
        let content = toml::to_string_pretty(&self).context("serialising config")?;
        std::fs::write(&cfg_path, &content).context("writing out config file")?;

        Ok(())
    }
}

pub fn root_path() -> Result<PathBuf> {
    let Some(root) = dotstore::home_store("moxen").context("initialising home store")? else {
        eprintln!("unable to get path to home directory");
        panic!("USE CUSTOM ERROR");
    };

    Ok(root)
}

pub fn load_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let root = root_path().context("loading home store directory")?;
    let path = root.join(path);

    Ok(path)
}

pub mod registry {
    use super::*;

    pub type MoxenRegistry = HashMap<i32, Addon>;

    pub fn initialise() -> Result<()> {
        for version in VERSIONS {
            let reg = MoxenRegistry::new();
            let registry_path = version.registry_path().context("registry path")?;
            let registry = serde_json::to_string_pretty(&reg).context("serialising registry")?;
            std::fs::write(&registry_path, &registry).context("writing new registry")?;
        }

        Ok(())
    }

    pub fn load(version: &GameVersion) -> Result<MoxenRegistry> {
        let registry_path = version.registry_path().context("registry path")?;
        let content = std::fs::read_to_string(&registry_path).context("reading registry file")?;
        let registry = serde_json::from_str(&content).context("deserialising registry")?;

        Ok(registry)
    }

    pub fn save(registry: &MoxenRegistry, version: &GameVersion) -> Result<()> {
        let registry_path = version.registry_path().context("registry path")?;
        let contents = serde_json::to_string_pretty(registry).context("serialising registry")?;
        std::fs::write(&registry_path, &contents).context("saving registry to disk")?;

        Ok(())
    }
}
