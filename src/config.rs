use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// AI client configuration from config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AiConfig {
    pub base_url: String,
    pub api_key_env: String,
    pub model: String,
    pub requests_per_minute: u32,
    pub max_retries: u32,
    pub connect_timeout_secs: u64,
    pub read_timeout_secs: u64,
    pub temperature: f64,
    pub max_tokens: u32,
    pub digest_max_bytes: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            api_key_env: "NVIDIA_API_KEY".to_string(),
            model: String::new(),
            requests_per_minute: 20,
            max_retries: 4,
            connect_timeout_secs: 10,
            read_timeout_secs: 120,
            temperature: 0.2,
            max_tokens: 1500,
            digest_max_bytes: 6000,
        }
    }
}

impl AiConfig {
    /// Returns the API key from the environment variable.
    /// Does NOT log or print the key value.
    pub fn api_key(&self) -> anyhow::Result<String> {
        std::env::var(&self.api_key_env)
            .with_context(|| format!("Please set the {} environment variable", self.api_key_env))
    }
}

/// Review configuration from config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReviewConfig {
    pub model: String,
    pub max_overview_chars: usize,
    pub jargon_terms: Vec<String>,
    pub deny_terms: Vec<String>,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_overview_chars: 700,
            jargon_terms: vec![
                "refactor".to_string(),
                "middleware".to_string(),
                "endpoint".to_string(),
                "pipeline".to_string(),
                "daemon".to_string(),
                "regex".to_string(),
                "boilerplate".to_string(),
                "scaffold".to_string(),
                "async".to_string(),
                "mutex".to_string(),
                "API".to_string(),
                "CLI".to_string(),
                "TUI".to_string(),
                "ORM".to_string(),
                "JSON".to_string(),
                "YAML".to_string(),
            ],
            deny_terms: vec![],
        }
    }
}

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
    /// AI configuration (M2: Enrich)
    #[serde(default)]
    pub ai: AiConfig,
    /// Review configuration (M3: Review)
    #[serde(default)]
    pub review: ReviewConfig,
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
            ai: AiConfig::default(),
            review: ReviewConfig::default(),
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

# AI Configuration (M2: Enrich)
# These settings control the AI enrichment feature.
[ai]
base_url = "https://integrate.api.nvidia.com/v1"
api_key_env = "NVIDIA_API_KEY"   # the NAME of the env var, never the key
model = ""                        # set by `ai models`
requests_per_minute = 20
max_retries = 4
connect_timeout_secs = 10
read_timeout_secs = 120
temperature = 0.2
max_tokens = 1500
digest_max_bytes = 6000

# Review Configuration (M3: Review)
# These settings control the AI review feature.
[review]
model = ""                  # must differ from [ai] model
max_overview_chars = 700
jargon_terms = ["refactor", "middleware", "endpoint", "pipeline", "daemon", "regex", "boilerplate", "scaffold", "async", "mutex", "API", "CLI", "TUI", "ORM", "JSON", "YAML"]
deny_terms = []             # the owner's private words; checked in code only, NEVER sent to any API
"#;
