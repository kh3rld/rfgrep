use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::fs; 
use dirs;
use toml;
use crate::cli::SearchMode;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(default)]
    pub search: SearchConfig,

    #[serde(default)]
    pub display: DisplayConfig,
    
    #[serde(default)]
    pub ignore: IgnoreConfig,
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
    // pub colors: ColorScheme, 
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
            patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
            ],
            binary_files: true,
            hidden_files: true,
            min_file_size: None,
        }
    }
}
impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_path()?;
        if let Some(path) = config_path {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            
            toml::from_str(&content)
                .with_context(|| "Failed to parse config file")
        } else {
            Ok(Self::default())
        }
    }

    fn find_config_path() -> Result<Option<PathBuf>> {
        // Check XDG config directory
        if let Some(xdg_config) = dirs::config_dir() {
            let xdg_path = xdg_config.join("rfgrep/config.toml");
            if xdg_path.exists() {
                return Ok(Some(xdg_path));
            }
        }

        // Check home directory
        if let Some(home) = dirs::home_dir() {
            let home_path = home.join(".rfgrep.toml");
            if home_path.exists() {
                return Ok(Some(home_path));
            }
        }

        // Check current directory
        let current_path = Path::new(".rfgrep.toml");
        if current_path.exists() {
            return Ok(Some(current_path.to_path_buf()));
        }

        Ok(None)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
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
                max_file_size: Some(100), // 100MB
                chunk_size: 100,
                parallel_jobs: None, // Auto-detect
                default_extensions: vec![],
            },
            display: DisplayConfig::default(),
            ignore: IgnoreConfig::default(),
        }
    }
}