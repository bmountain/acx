use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub const PROJECT_CONFIG_FILE: &str = "acx.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_contests_dir")]
    pub contests_dir: String,
    pub default_profile: String,
    #[serde(default)]
    pub solve: SolveConfig,
    #[serde(default)]
    pub template: TemplateConfig,
    pub profiles: BTreeMap<String, BuildProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildProfile {
    pub source: String,
    pub binary: String,
    pub build: String,
    pub run: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub enabled_on_download: bool,
    pub output: String,
    pub overwrite: bool,
    pub fallback_on_parse_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveConfig {
    pub command: String,
}

fn default_contests_dir() -> String {
    "contests".to_string()
}

impl Default for SolveConfig {
    fn default() -> Self {
        Self {
            command: "nvim -O {statement} {source}".to_string(),
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            enabled_on_download: false,
            output: "main.cpp".to_string(),
            overwrite: false,
            fallback_on_parse_error: true,
        }
    }
}

impl ProjectConfig {
    pub fn load_from_current_dir() -> Result<(Self, PathBuf)> {
        let start = std::env::current_dir().context("failed to read current directory")?;
        if let Some(path) = find_project_config(&start) {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("failed to read project config: {}", path.display()))?;
            let config = serde_json::from_str(&text)
                .with_context(|| format!("failed to parse project config: {}", path.display()))?;
            let root = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            return Ok((config, root));
        }

        Ok((Self::default(), start))
    }

    pub fn profile(&self, name: Option<&str>) -> Result<(String, &BuildProfile)> {
        let name = name.unwrap_or(&self.default_profile);
        let profile = self
            .profiles
            .get(name)
            .with_context(|| format!("build profile not found: {name}"))?;
        Ok((name.to_string(), profile))
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "debug".to_string(),
            BuildProfile {
                source: "main.cpp".to_string(),
                binary: "main_debug".to_string(),
                build: "g++ -std=gnu++23 -O0 -g -Wall -Wextra -DLOCAL -fsanitize=address,undefined -fno-omit-frame-pointer -o {binary} {source}".to_string(),
                run: "./{binary}".to_string(),
            },
        );
        profiles.insert(
            "release".to_string(),
            BuildProfile {
                source: "main.cpp".to_string(),
                binary: "main".to_string(),
                build: "g++ -std=gnu++23 -O2 -Wall -Wextra -o {binary} {source}".to_string(),
                run: "./{binary}".to_string(),
            },
        );

        Self {
            contests_dir: default_contests_dir(),
            default_profile: "debug".to_string(),
            solve: SolveConfig::default(),
            template: TemplateConfig::default(),
            profiles,
        }
    }
}

fn find_project_config(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(current) = dir {
        let path = current.join(PROJECT_CONFIG_FILE);
        if path.exists() {
            return Some(path);
        }
        dir = current.parent();
    }
    None
}
