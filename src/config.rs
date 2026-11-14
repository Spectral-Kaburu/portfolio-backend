use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for the portfolio engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Root directories to scan for git repositories.
    pub roots: Vec<PathBuf>,
    /// Maximum depth to search for repositories under each root.
    pub max_depth: usize,
    /// Number of days to consider a repository "active".
    pub active_days: u32,
    /// Directory names to exclude during scanning.
    pub exclude_dirs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            max_depth: 4,
            active_days: 30,
            exclude_dirs: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".venv".to_string(),
                "vendor".to_string(),
            ],
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a TOML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if path.exists() {
            anyhow::bail!(
                "Config file already exists at: {}. Refusing to overwrite.",
                path.display()
            );
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    /// Save configuration, overwriting if it exists (with confirmation handled by caller).
    #[allow(dead_code)]
    pub fn save_overwrite(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }

    /// Validate the configuration.
    fn validate(&self) -> Result<()> {
        if self.roots.is_empty() {
            anyhow::bail!("Configuration must have at least one root directory");
        }
        if self.max_depth == 0 {
            anyhow::bail!("max_depth must be at least 1");
        }
        if self.active_days == 0 {
            anyhow::bail!("active_days must be at least 1");
        }
        for root in &self.roots {
            if !root.exists() {
                anyhow::bail!("Root directory does not exist: {}", root.display());
            }
            if !root.is_dir() {
                anyhow::bail!("Root path is not a directory: {}", root.display());
            }
        }
        Ok(())
    }
}

/// Example configuration content for config.example.toml.
pub const EXAMPLE_CONFIG: &str = r#"# Portfolio Engine Configuration
# Copy this file to config.toml and adjust the paths.

# Root directories to scan for git repositories.
# Use absolute paths. Tilde (~) expansion is not supported.
roots = [
    "/home/user/projects",
    "/home/user/work",
]

# How deep to look for repositories under each root (default: 4).
max_depth = 4

# Repositories with a commit within this many days are considered "active" (default: 30).
active_days = 30

# Directory names to skip during scanning (default: common build/dependency dirs).
exclude_dirs = [
    "node_modules",
    "target",
    ".venv",
    "vendor",
]
"#;
