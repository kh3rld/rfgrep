use crate::cli::SearchMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(default)]
    pub search: SearchConfig,

    #[serde(default)]
    pub display: DisplayConfig,

    #[serde(default)]
    pub ignore: IgnoreConfig,

    #[serde(default)]
    pub performance: PerformanceConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchConfig {
    pub default_mode: SearchMode,
    pub context_lines: usize,
    pub max_file_size: Option<u64>,
    pub chunk_size: usize,
    pub parallel_jobs: Option<usize>,
    pub default_extensions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(skip)]
    pub progress_style: Option<indicatif::ProgressStyle>,
    pub show_timing: bool,
    pub show_summary: bool,
}

impl fmt::Debug for DisplayConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DisplayConfig")
            .field("progress_style", &self.progress_style.is_some())
            .field("show_timing", &self.show_timing)
            .field("show_summary", &self.show_summary)
            .finish()
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            progress_style: None,
            show_timing: true,
            show_summary: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IgnoreConfig {
    pub patterns: Vec<String>,
    pub binary_files: bool,
    pub hidden_files: bool,
    pub min_file_size: Option<u64>,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            patterns: vec!["node_modules".to_string(), ".git".to_string()],
            binary_files: true,
            hidden_files: true,
            min_file_size: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_mmap_threshold")]
    pub mmap_threshold_mb: u64,
    #[serde(default = "default_max_memory")]
    pub max_memory_usage_mb: u64,
    #[serde(default = "default_chunk_multiplier")]
    pub chunk_size_multiplier: f64,
    #[serde(default = "default_adaptive_memory")]
    pub adaptive_memory: bool,
    #[serde(default = "default_regex_cache_size")]
    pub regex_cache_size: usize,
    #[serde(default = "default_metadata_cache_size")]
    pub metadata_cache_size: usize,
}

fn default_mmap_threshold() -> u64 {
    16
}
fn default_max_memory() -> u64 {
    512
}
fn default_chunk_multiplier() -> f64 {
    1.0
}
fn default_adaptive_memory() -> bool {
    true
}
fn default_regex_cache_size() -> usize {
    100
}
fn default_metadata_cache_size() -> usize {
    1000
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            mmap_threshold_mb: default_mmap_threshold(),
            max_memory_usage_mb: default_max_memory(),
            chunk_size_multiplier: default_chunk_multiplier(),
            adaptive_memory: default_adaptive_memory(),
            regex_cache_size: default_regex_cache_size(),
            metadata_cache_size: default_metadata_cache_size(),
        }
    }
}

impl Config {
    #[allow(dead_code)]
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_path()?;
        if let Some(path) = config_path {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;

            toml::from_str(&content).with_context(|| "Failed to parse config file")
        } else {
            Ok(Self::default())
        }
    }

    #[allow(dead_code)]
    fn find_config_path() -> Result<Option<PathBuf>> {
        if let Some(xdg_config) = dirs::config_dir() {
            let xdg_path = xdg_config.join("rfgrep/config.toml");
            if xdg_path.exists() {
                return Ok(Some(xdg_path));
            }
        }

        if let Some(home) = dirs::home_dir() {
            let home_path = home.join(".rfgrep.toml");
            if home_path.exists() {
                return Ok(Some(home_path));
            }
        }

        let current_path = Path::new(".rfgrep.toml");
        if current_path.exists() {
            return Ok(Some(current_path.to_path_buf()));
        }

        Ok(None)
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search: SearchConfig {
                default_mode: SearchMode::Regex,
                context_lines: 2,
                max_file_size: Some(100),
                chunk_size: 100,
                parallel_jobs: None,
                default_extensions: vec![],
            },
            display: DisplayConfig::default(),
            ignore: IgnoreConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}
