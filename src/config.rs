use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub revel_session: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            if let Some(config) = load_legacy_config()? {
                return Ok(config);
            }
            return Ok(Self::default());
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        serde_json::from_str(&text).context("failed to parse config JSON")
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory: {}", parent.display())
            })?;
        }

        let text = serde_json::to_string_pretty(self)?;
        fs::write(&path, text)
            .with_context(|| format!("failed to write config file: {}", path.display()))
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("failed to locate config directory")?;
    Ok(base.join("acx").join("config.json"))
}

fn load_legacy_config() -> Result<Option<Config>> {
    let base = dirs::config_dir().context("failed to locate config directory")?;
    let path = base.join("atcoder-rust").join("config.json");
    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read legacy config file: {}", path.display()))?;
    let config = serde_json::from_str(&text).context("failed to parse legacy config JSON")?;
    Ok(Some(config))
}
