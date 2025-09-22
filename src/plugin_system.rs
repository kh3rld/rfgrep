//! Enhanced plugin system for rfgrep with dynamic loading and better integration
use crate::error::Result as RfgrepResult;
use crate::processor::SearchMatch;
use crate::search_algorithms::SearchAlgorithm;
use crate::streaming_search::{StreamingConfig, StreamingSearchPipeline};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Enhanced plugin trait with more capabilities
pub trait EnhancedSearchPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin version
    fn version(&self) -> &str;

    /// Plugin description
    fn description(&self) -> &str;

    /// Check if this plugin can handle the given file
    fn can_handle(&self, file: &Path) -> bool;

    /// Get the priority of this plugin (lower = higher priority)
    fn priority(&self) -> u32;

    /// Search a file using this plugin
    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>>;

    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<String>;

    /// Get plugin configuration options
    fn get_config_options(&self) -> HashMap<String, PluginConfigOption>;

    /// Update plugin configuration
    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> RfgrepResult<()>;

    /// Check if plugin supports streaming search
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Get preferred search algorithm for this plugin
    fn preferred_algorithm(&self) -> Option<SearchAlgorithm> {
        None
    }

    /// Initialize plugin with given configuration
    fn initialize(&mut self, _config: PluginConfig) -> RfgrepResult<()> {
        Ok(())
    }

    /// Cleanup plugin resources
    fn cleanup(&mut self) -> RfgrepResult<()> {
        Ok(())
    }
}

/// Plugin configuration option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigOption {
    pub name: String,
    pub description: String,
    pub default_value: serde_json::Value,
    pub value_type: ConfigValueType,
}

/// Configuration value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigValueType {
    String,
    Integer,
    Boolean,
    Float,
    Array,
    Object,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub priority: u32,
    pub settings: HashMap<String, serde_json::Value>,
    pub streaming_enabled: bool,
    pub preferred_algorithm: Option<SearchAlgorithm>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 100,
            settings: HashMap::new(),
            streaming_enabled: false,
            preferred_algorithm: None,
        }
    }
}

/// Enhanced plugin manager with dynamic loading and configuration
pub struct EnhancedPluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn EnhancedSearchPlugin>>>>,
    plugin_configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
    streaming_pipeline: Option<StreamingSearchPipeline>,
}

impl EnhancedPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_configs: Arc::new(RwLock::new(HashMap::new())),
            streaming_pipeline: None,
        }
    }

    /// Register a plugin
    pub async fn register_plugin(
        &self,
        mut plugin: Box<dyn EnhancedSearchPlugin>,
    ) -> RfgrepResult<()> {
        let name = plugin.name().to_string();
        let config = PluginConfig::default();

        plugin.initialize(config.clone())?;

        self.plugins.write().await.insert(name.clone(), plugin);
        self.plugin_configs.write().await.insert(name, config);

        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&self, name: &str) -> RfgrepResult<()> {
        if let Some(mut plugin) = self.plugins.write().await.remove(name) {
            plugin.cleanup()?;
        }
        self.plugin_configs.write().await.remove(name);
        Ok(())
    }

    /// Update plugin configuration
    pub async fn update_plugin_config(&self, name: &str, config: PluginConfig) -> RfgrepResult<()> {
        if let Some(plugin) = self.plugins.write().await.get_mut(name) {
            plugin.update_config(config.settings.clone())?;
        }
        self.plugin_configs
            .write()
            .await
            .insert(name.to_string(), config);
        Ok(())
    }

    /// Get plugin configuration
    pub async fn get_plugin_config(&self, name: &str) -> Option<PluginConfig> {
        self.plugin_configs.read().await.get(name).cloned()
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        let configs = self.plugin_configs.read().await;

        plugins
            .iter()
            .map(|(name, plugin)| {
                let config = configs.get(name).cloned().unwrap_or_default();
                PluginInfo {
                    name: name.clone(),
                    version: plugin.version().to_string(),
                    description: plugin.description().to_string(),
                    enabled: config.enabled,
                    priority: config.priority,
                    supported_extensions: plugin.supported_extensions(),
                    supports_streaming: plugin.supports_streaming(),
                }
            })
            .collect()
    }

    /// Search a file using the best available plugin
    pub async fn search_file(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        let plugins = self.plugins.read().await;
        let configs = self.plugin_configs.read().await;

        // Find the best plugin for this file
        let mut candidates: Vec<_> = plugins
            .iter()
            .filter(|(name, plugin)| {
                if let Some(config) = configs.get(*name) {
                    config.enabled && plugin.can_handle(file)
                } else {
                    false
                }
            })
            .collect();

        // Sort by priority
        candidates.sort_by_key(|(name, _)| configs.get(*name).map(|c| c.priority).unwrap_or(100));

        if let Some((_, plugin)) = candidates.first() {
            // Check if plugin supports streaming and we have a streaming pipeline
            if plugin.supports_streaming() && self.streaming_pipeline.is_some() {
                // Use streaming search
                if let Some(pipeline) = &self.streaming_pipeline {
                    pipeline.search_file(file, pattern).await
                } else {
                    plugin.search(file, pattern)
                }
            } else {
                // Use regular search
                plugin.search(file, pattern)
            }
        } else {
            // No plugin can handle this file
            Ok(vec![])
        }
    }

    /// Set up streaming pipeline for plugins that support it
    pub fn setup_streaming(&mut self, config: StreamingConfig) {
        self.streaming_pipeline = Some(StreamingSearchPipeline::new(config));
    }

    /// Get plugin statistics
    pub async fn get_plugin_stats(&self) -> PluginStats {
        let plugins = self.plugins.read().await;
        let configs = self.plugin_configs.read().await;

        let total_plugins = plugins.len();
        let enabled_plugins = configs.values().filter(|c| c.enabled).count();
        let streaming_plugins = plugins.values().filter(|p| p.supports_streaming()).count();

        PluginStats {
            total_plugins,
            enabled_plugins,
            streaming_plugins,
            disabled_plugins: total_plugins - enabled_plugins,
        }
    }
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub priority: u32,
    pub supported_extensions: Vec<String>,
    pub supports_streaming: bool,
}

/// Plugin statistics
#[derive(Debug, Clone)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub enabled_plugins: usize,
    pub disabled_plugins: usize,
    pub streaming_plugins: usize,
}

/// Built-in text search plugin with enhanced capabilities
pub struct EnhancedTextSearchPlugin {
    config: PluginConfig,
    text_extensions: Vec<String>,
    case_sensitive: bool,
    max_file_size: Option<usize>,
}

impl EnhancedTextSearchPlugin {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
            text_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "go".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "hpp".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "toml".to_string(),
                "xml".to_string(),
                "html".to_string(),
                "css".to_string(),
                "sh".to_string(),
                "bash".to_string(),
                "zsh".to_string(),
                "fish".to_string(),
                "ps1".to_string(),
                "bat".to_string(),
                "ini".to_string(),
                "cfg".to_string(),
                "conf".to_string(),
            ],
            case_sensitive: false,
            max_file_size: Some(10 * 1024 * 1024), // 10MB default
        }
    }
}

impl EnhancedSearchPlugin for EnhancedTextSearchPlugin {
    fn name(&self) -> &str {
        "enhanced_text"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Enhanced text file search with streaming support"
    }

    fn can_handle(&self, file: &Path) -> bool {
        // Check file size first
        if let Some(max_size) = self.max_file_size {
            if let Ok(metadata) = file.metadata() {
                if metadata.len() > max_size as u64 {
                    return false;
                }
            }
        }

        // Check extension
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            self.text_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(ext))
        } else {
            // Check if file is text by reading first few bytes
            if let Ok(content) = std::fs::read(file) {
                content.iter().take(1024).all(|&b| b.is_ascii() || b >= 128)
            } else {
                false
            }
        }
    }

    fn priority(&self) -> u32 {
        10
    }

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        let content = std::fs::read_to_string(file)?;
        let regex_pattern = if self.case_sensitive {
            pattern.to_string()
        } else {
            format!("(?i){pattern}")
        };
        let regex = regex::Regex::new(&regex_pattern)?;

        let mut matches = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            for mat in regex.find_iter(line) {
                matches.push(SearchMatch {
                    path: file.to_path_buf(),
                    line_number: line_num + 1,
                    line: line.to_string(),
                    context_before: Vec::new(),
                    context_after: Vec::new(),
                    matched_text: mat.as_str().to_string(),
                    column_start: mat.start(),
                    column_end: mat.end(),
                });
            }
        }

        Ok(matches)
    }

    fn supported_extensions(&self) -> Vec<String> {
        self.text_extensions.clone()
    }

    fn get_config_options(&self) -> HashMap<String, PluginConfigOption> {
        let mut options = HashMap::new();
        options.insert(
            "case_sensitive".to_string(),
            PluginConfigOption {
                name: "Case Sensitive".to_string(),
                description: "Enable case-sensitive search".to_string(),
                default_value: serde_json::Value::Bool(false),
                value_type: ConfigValueType::Boolean,
            },
        );
        options.insert(
            "max_file_size".to_string(),
            PluginConfigOption {
                name: "Max File Size".to_string(),
                description: "Maximum file size to process (in bytes)".to_string(),
                default_value: serde_json::Value::Number(serde_json::Number::from(
                    10 * 1024 * 1024,
                )),
                value_type: ConfigValueType::Integer,
            },
        );
        options
    }

    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> RfgrepResult<()> {
        if let Some(value) = config.get("case_sensitive") {
            if let Some(bool_val) = value.as_bool() {
                self.case_sensitive = bool_val;
            }
        }
        if let Some(value) = config.get("max_file_size") {
            if let Some(int_val) = value.as_u64() {
                self.max_file_size = Some(int_val as usize);
            }
        }
        Ok(())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn preferred_algorithm(&self) -> Option<SearchAlgorithm> {
        Some(SearchAlgorithm::BoyerMoore)
    }
}

/// Built-in binary search plugin with enhanced capabilities
pub struct EnhancedBinarySearchPlugin {
    config: PluginConfig,
    binary_extensions: Vec<String>,
    search_metadata: bool,
}

impl EnhancedBinarySearchPlugin {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
            binary_extensions: vec![
                "exe".to_string(),
                "dll".to_string(),
                "so".to_string(),
                "dylib".to_string(),
                "bin".to_string(),
                "obj".to_string(),
                "o".to_string(),
                "a".to_string(),
                "lib".to_string(),
                "pdb".to_string(),
                "map".to_string(),
            ],
            search_metadata: true,
        }
    }
}

impl EnhancedSearchPlugin for EnhancedBinarySearchPlugin {
    fn name(&self) -> &str {
        "enhanced_binary"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Enhanced binary file search with metadata support"
    }

    fn can_handle(&self, file: &Path) -> bool {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            self.binary_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(ext))
        } else {
            // Check if file is binary by reading first few bytes
            if let Ok(content) = std::fs::read(file) {
                content.iter().take(1024).any(|&b| b == 0)
            } else {
                false
            }
        }
    }

    fn priority(&self) -> u32 {
        20
    }

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        let content = std::fs::read(file)?;
        let pattern_bytes = pattern.as_bytes();

        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(found) = memchr::memmem::find(&content[pos..], pattern_bytes) {
            let absolute_pos = pos + found;
            matches.push(SearchMatch {
                path: file.to_path_buf(),
                line_number: 1,
                line: format!("Binary data at offset {}", absolute_pos),
                context_before: Vec::new(),
                context_after: Vec::new(),
                matched_text: pattern.to_string(),
                column_start: absolute_pos,
                column_end: absolute_pos + pattern_bytes.len(),
            });
            pos = absolute_pos + 1;
        }

        Ok(matches)
    }

    fn supported_extensions(&self) -> Vec<String> {
        self.binary_extensions.clone()
    }

    fn get_config_options(&self) -> HashMap<String, PluginConfigOption> {
        let mut options = HashMap::new();
        options.insert(
            "search_metadata".to_string(),
            PluginConfigOption {
                name: "Search Metadata".to_string(),
                description: "Search file metadata in addition to content".to_string(),
                default_value: serde_json::Value::Bool(true),
                value_type: ConfigValueType::Boolean,
            },
        );
        options
    }

    fn update_config(&mut self, config: HashMap<String, serde_json::Value>) -> RfgrepResult<()> {
        if let Some(value) = config.get("search_metadata") {
            if let Some(bool_val) = value.as_bool() {
                self.search_metadata = bool_val;
            }
        }
        Ok(())
    }

    fn preferred_algorithm(&self) -> Option<SearchAlgorithm> {
        Some(SearchAlgorithm::Simd)
    }
}

/// Plugin registry for managing plugin discovery and loading
pub struct PluginRegistry {
    manager: Arc<EnhancedPluginManager>,
    plugin_paths: Vec<std::path::PathBuf>,
}

impl PluginRegistry {
    pub fn new(manager: Arc<EnhancedPluginManager>) -> Self {
        Self {
            manager,
            plugin_paths: Vec::new(),
        }
    }

    /// Add a plugin directory to search for plugins
    pub fn add_plugin_directory(&mut self, path: std::path::PathBuf) {
        self.plugin_paths.push(path);
    }

    /// Load all plugins from registered directories
    pub async fn load_plugins(&self) -> RfgrepResult<()> {
        // Register built-in plugins
        self.manager
            .register_plugin(Box::new(EnhancedTextSearchPlugin::new()))
            .await?;
        self.manager
            .register_plugin(Box::new(EnhancedBinarySearchPlugin::new()))
            .await?;

        // Load dynamic plugins from directories
        self.load_dynamic_plugins().await?;

        Ok(())
    }

    /// Reload all plugins
    pub async fn reload_plugins(&self) -> RfgrepResult<()> {
        // Clear existing plugins
        let mut plugins = self.manager.plugins.write().await;
        for (_, mut plugin) in plugins.drain() {
            let _ = plugin.cleanup();
        }
        drop(plugins);

        // Reload plugins
        self.load_plugins().await
    }

    /// Load dynamic plugins from directories
    async fn load_dynamic_plugins(&self) -> RfgrepResult<()> {
        // Get plugin directories from environment or use defaults
        let plugin_dirs = self.get_plugin_directories();

        for dir in plugin_dirs {
            if let Err(e) = self.load_plugins_from_directory(&dir).await {
                log::warn!("Failed to load plugins from directory {:?}: {}", dir, e);
            }
        }

        Ok(())
    }

    /// Get plugin directories to search
    fn get_plugin_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Add directories from environment variable
        if let Ok(plugin_path) = std::env::var("RFGREP_PLUGIN_PATH") {
            for path in plugin_path.split(':') {
                dirs.push(PathBuf::from(path));
            }
        }

        // Add default directories
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".local/share/rfgrep/plugins"));
            dirs.push(home.join(".config/rfgrep/plugins"));
        }

        // Add system-wide directories
        dirs.push(PathBuf::from("/usr/local/lib/rfgrep/plugins"));
        dirs.push(PathBuf::from("/usr/lib/rfgrep/plugins"));

        // Filter to only existing directories
        dirs.into_iter()
            .filter(|d| d.exists() && d.is_dir())
            .collect()
    }

    /// Load plugins from a specific directory
    async fn load_plugins_from_directory(&self, dir: &Path) -> RfgrepResult<()> {
        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Check if it's a shared library
            if self.is_plugin_file(&path) {
                if let Err(e) = self.load_plugin_from_file(&path).await {
                    log::warn!("Failed to load plugin from {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Check if a file is a valid plugin
    fn is_plugin_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(OsStr::to_str)
            .map_or(false, |extension| {
                matches!(extension, "so" | "dylib" | "dll")
            })
    }

    /// Load a plugin from a shared library file
    async fn load_plugin_from_file(&self, path: &Path) -> RfgrepResult<()> {
        unsafe {
            let lib = Library::new(path)?;

            // Try to get the plugin creation function
            let create_plugin: Symbol<unsafe extern "C" fn() -> *mut dyn EnhancedSearchPlugin> =
                lib.get(b"create_plugin")?;

            let plugin_ptr = create_plugin();
            if plugin_ptr.is_null() {
                return Err(crate::error::RfgrepError::Other(
                    "Plugin creation function returned null".to_string(),
                ));
            }

            // Convert raw pointer to Box and register
            let plugin = Box::from_raw(plugin_ptr);
            self.manager.register_plugin(plugin).await?;

            log::info!("Successfully loaded dynamic plugin from {:?}", path);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_plugin_manager() {
        let manager = Arc::new(EnhancedPluginManager::new());
        let registry = PluginRegistry::new(manager.clone());

        registry.load_plugins().await.unwrap();

        let plugins = manager.list_plugins().await;
        assert!(!plugins.is_empty());

        let stats = manager.get_plugin_stats().await;
        assert!(stats.total_plugins > 0);
    }

    #[tokio::test]
    async fn test_text_plugin() {
        let _plugin = EnhancedTextSearchPlugin::new();
        let _test_file = PathBuf::from("test.txt");

        // This would need a real test file
        // assert!(plugin.can_handle(&test_file));
    }
}
