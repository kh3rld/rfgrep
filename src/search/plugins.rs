//! Plugin system for extensible search capabilities
use crate::error::Result as RfgrepResult;
use crate::processor::SearchMatch;
use std::collections::HashMap;
use std::path::Path;

/// Trait for search plugins
pub trait SearchPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn can_handle(&self, file: &Path) -> bool;
    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>>;
    fn priority(&self) -> u32 {
        100
    }
}

/// Plugin manager that coordinates different search plugins
pub struct PluginManager {
    plugins: Vec<Box<dyn SearchPlugin>>,
    plugin_cache: HashMap<String, usize>,
}

impl PluginManager {
    pub fn new() -> RfgrepResult<Self> {
        let mut manager = Self {
            plugins: Vec::new(),
            plugin_cache: HashMap::new(),
        };

        // Register built-in plugins
        manager.register_plugin(Box::new(TextSearchPlugin::new()?));
        manager.register_plugin(Box::new(BinarySearchPlugin::new()?));
        manager.register_plugin(Box::new(ArchiveSearchPlugin::new()?));
        manager.register_plugin(Box::new(DatabaseSearchPlugin::new()?));
        manager.register_plugin(Box::new(ImageSearchPlugin::new()?));

        Ok(manager)
    }

    pub fn register_plugin(&mut self, plugin: Box<dyn SearchPlugin>) {
        let name = plugin.name().to_string();
        let index = self.plugins.len();
        self.plugins.push(plugin);
        self.plugin_cache.insert(name, index);
    }

    pub fn search_file(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        // Find the best plugin for this file
        let mut candidates: Vec<_> = self
            .plugins
            .iter()
            .enumerate()
            .filter(|(_, plugin)| plugin.can_handle(file))
            .collect();

        // Sort by priority (lower number = higher priority)
        candidates.sort_by_key(|(_, plugin)| plugin.priority());

        if let Some((_, plugin)) = candidates.first() {
            plugin.search(file, pattern)
        } else {
            // Fallback to text search
            Ok(vec![])
        }
    }

    pub fn get_plugin_names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name()).collect()
    }
}

/// Text file search plugin
pub struct TextSearchPlugin {
    text_extensions: Vec<String>,
}

impl TextSearchPlugin {
    pub fn new() -> RfgrepResult<Self> {
        Ok(Self {
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
            ],
        })
    }
}

impl SearchPlugin for TextSearchPlugin {
    fn name(&self) -> &str {
        "text"
    }

    fn can_handle(&self, file: &Path) -> bool {
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

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        let content = std::fs::read_to_string(file)?;
        let regex = regex::Regex::new(pattern)?;

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

    fn priority(&self) -> u32 {
        10
    }
}

/// Binary file search plugin
pub struct BinarySearchPlugin {
    binary_extensions: Vec<String>,
}

impl BinarySearchPlugin {
    pub fn new() -> RfgrepResult<Self> {
        Ok(Self {
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
            ],
        })
    }
}

impl SearchPlugin for BinarySearchPlugin {
    fn name(&self) -> &str {
        "binary"
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

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        let content = std::fs::read(file)?;
        let pattern_bytes = pattern.as_bytes();

        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(found) = memchr::memmem::find(&content[pos..], pattern_bytes) {
            let absolute_pos = pos + found;
            matches.push(SearchMatch {
                path: file.to_path_buf(),
                line_number: 1, // Binary files don't have line numbers
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

    fn priority(&self) -> u32 {
        20
    }
}

/// Archive file search plugin
pub struct ArchiveSearchPlugin {
    archive_extensions: Vec<String>,
}

impl ArchiveSearchPlugin {
    pub fn new() -> RfgrepResult<Self> {
        Ok(Self {
            archive_extensions: vec![
                "zip".to_string(),
                "tar".to_string(),
                "gz".to_string(),
                "bz2".to_string(),
                "xz".to_string(),
                "7z".to_string(),
                "rar".to_string(),
                "tar.gz".to_string(),
                "tgz".to_string(),
            ],
        })
    }
}

impl SearchPlugin for ArchiveSearchPlugin {
    fn name(&self) -> &str {
        "archive"
    }

    fn can_handle(&self, file: &Path) -> bool {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            self.archive_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(ext))
        } else {
            false
        }
    }

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        // For now, just search the archive metadata
        // In a full implementation, this would extract and search contents
        let content = std::fs::read(file)?;
        let pattern_bytes = pattern.as_bytes();

        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(found) = memchr::memmem::find(&content[pos..], pattern_bytes) {
            let absolute_pos = pos + found;
            matches.push(SearchMatch {
                path: file.to_path_buf(),
                line_number: 1,
                line: format!("Archive metadata at offset {}", absolute_pos),
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

    fn priority(&self) -> u32 {
        30
    }
}

/// Database file search plugin
pub struct DatabaseSearchPlugin {
    db_extensions: Vec<String>,
}

impl DatabaseSearchPlugin {
    pub fn new() -> RfgrepResult<Self> {
        Ok(Self {
            db_extensions: vec![
                "sqlite".to_string(),
                "db".to_string(),
                "sqlite3".to_string(),
                "mdb".to_string(),
                "accdb".to_string(),
            ],
        })
    }
}

impl SearchPlugin for DatabaseSearchPlugin {
    fn name(&self) -> &str {
        "database"
    }

    fn can_handle(&self, file: &Path) -> bool {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            self.db_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(ext))
        } else {
            false
        }
    }

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        // For now, just search the database file as binary
        // In a full implementation, this would connect to the database and search
        let content = std::fs::read(file)?;
        let pattern_bytes = pattern.as_bytes();

        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(found) = memchr::memmem::find(&content[pos..], pattern_bytes) {
            let absolute_pos = pos + found;
            matches.push(SearchMatch {
                path: file.to_path_buf(),
                line_number: 1,
                line: format!("Database content at offset {}", absolute_pos),
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

    fn priority(&self) -> u32 {
        40
    }
}

/// Image file search plugin
pub struct ImageSearchPlugin {
    image_extensions: Vec<String>,
}

impl ImageSearchPlugin {
    pub fn new() -> RfgrepResult<Self> {
        Ok(Self {
            image_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "bmp".to_string(),
                "webp".to_string(),
                "svg".to_string(),
                "ico".to_string(),
                "tiff".to_string(),
            ],
        })
    }
}

impl SearchPlugin for ImageSearchPlugin {
    fn name(&self) -> &str {
        "image"
    }

    fn can_handle(&self, file: &Path) -> bool {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            self.image_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(ext))
        } else {
            false
        }
    }

    fn search(&self, file: &Path, pattern: &str) -> RfgrepResult<Vec<SearchMatch>> {
        // Search image metadata (EXIF, etc.)
        let content = std::fs::read(file)?;
        let pattern_bytes = pattern.as_bytes();

        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(found) = memchr::memmem::find(&content[pos..], pattern_bytes) {
            let absolute_pos = pos + found;
            matches.push(SearchMatch {
                path: file.to_path_buf(),
                line_number: 1,
                line: format!("Image metadata at offset {}", absolute_pos),
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

    fn priority(&self) -> u32 {
        50
    }
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub plugin_settings: HashMap<String, serde_json::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled_plugins: vec![
                "text".to_string(),
                "binary".to_string(),
                "archive".to_string(),
                "database".to_string(),
            ],
            plugin_settings: HashMap::new(),
        }
    }
}
