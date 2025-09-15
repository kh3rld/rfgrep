// Removed unused imports
use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::path::Path;

/// Search decision for a file
#[derive(Debug, Clone)]
pub enum SearchDecision {
    Search(SearchMode),
    Skip(String),
    Conditional(SearchMode, String),
}

/// Different search modes for different file types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    FullText,   // Search entire file content
    Metadata,   // Search only metadata/headers
    Filename,   // Search only filename
    Structured, // Search structured content (JSON, XML)
}

/// File type classifier that determines how to handle different file types
pub struct FileTypeClassifier {
    always_search: HashSet<String>,
    conditional_search: HashSet<String>,
    skip_by_default: HashSet<String>,
    never_search: HashSet<String>,
    size_limits: HashMap<String, u64>,
    search_modes: HashMap<String, SearchMode>,
}

impl FileTypeClassifier {
    pub fn new() -> Self {
        Self {
            always_search: [
                // Plain text
                "txt",
                "md",
                "rst",
                "org",
                "tex",
                "log",
                "readme",
                "changelog",
                // Source code
                "rs",
                "py",
                "js",
                "ts",
                "go",
                "java",
                "cpp",
                "c",
                "h",
                "hpp",
                "cs",
                "php",
                "rb",
                "swift",
                "kt",
                "scala",
                "dart",
                "r",
                "lua",
                "sh",
                "bash",
                "zsh",
                "fish",
                "ps1",
                "bat",
                "cmd",
                // Web technologies
                "html",
                "htm",
                "css",
                "scss",
                "sass",
                "less",
                "vue",
                "jsx",
                "tsx",
                // Configuration
                "json",
                "yaml",
                "yml",
                "toml",
                "ini",
                "cfg",
                "conf",
                "config",
                "xml",
                "svg",
                "env",
                "properties",
                "dockerfile",
                "makefile",
                // Data formats
                "csv",
                "tsv",
                "sql",
                "graphql",
                "gql",
                // Documentation
                "adoc",
                "asciidoc",
                "tex",
                "latex",
                "pod",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>(),

            conditional_search: [
                // Structured documents (search metadata)
                "pdf", "docx", "xlsx", "pptx", "odt", "ods", "odp",
                // Archives (search filenames)
                "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "cab", // Databases
                "db", "sqlite", "mdb", "accdb", // Media with metadata
                "mp3", "flac", "wav", "aac", "ogg", "wma", "mp4", "mkv", "mov", "avi", "flv",
                "wmv", "webm", "m4v", // Images with metadata
                "jpg", "jpeg", "png", "gif", "bmp", "webp", "ico", "tiff",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>(),

            skip_by_default: [
                // Executables
                "exe", "dll", "so", "dylib", "appimage", "bin", "class", "jar", "msi", "deb", "rpm",
                "snap", "apk", "ipa", // System files
                "iso", "img", "vdi", "vmdk", "qcow2", "raw", // Temporary files
                "tmp", "temp", "swp", "bak", "backup", "lock", "pid",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>(),

            never_search: [
                // Dangerous/irrelevant
                "enc", "gpg", "asc", "sig", "key", "pem", "p12", "pfx", "core", "dump", "crash",
                "hs_err",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>(),

            size_limits: HashMap::from([
                // Text files - generous limits
                ("txt".to_string(), 100 * 1024 * 1024), // 100MB
                ("md".to_string(), 50 * 1024 * 1024),   // 50MB
                ("rs".to_string(), 50 * 1024 * 1024),   // 50MB
                ("py".to_string(), 50 * 1024 * 1024),   // 50MB
                ("js".to_string(), 50 * 1024 * 1024),   // 50MB
                ("json".to_string(), 50 * 1024 * 1024), // 50MB
                ("xml".to_string(), 50 * 1024 * 1024),  // 50MB
                ("html".to_string(), 50 * 1024 * 1024), // 50MB
                ("css".to_string(), 50 * 1024 * 1024),  // 50MB
                ("log".to_string(), 200 * 1024 * 1024), // 200MB
                // Structured files - moderate limits
                ("pdf".to_string(), 10 * 1024 * 1024),  // 10MB
                ("docx".to_string(), 10 * 1024 * 1024), // 10MB
                ("xlsx".to_string(), 10 * 1024 * 1024), // 10MB
                ("pptx".to_string(), 10 * 1024 * 1024), // 10MB
                // Archives - moderate limits
                ("zip".to_string(), 20 * 1024 * 1024), // 20MB
                ("tar".to_string(), 20 * 1024 * 1024), // 20MB
                ("gz".to_string(), 20 * 1024 * 1024),  // 20MB
                ("7z".to_string(), 20 * 1024 * 1024),  // 20MB
                // Media files - strict limits
                ("mp4".to_string(), 5 * 1024 * 1024), // 5MB
                ("mp3".to_string(), 5 * 1024 * 1024), // 5MB
                ("jpg".to_string(), 2 * 1024 * 1024), // 2MB
                ("png".to_string(), 2 * 1024 * 1024), // 2MB
            ]),

            search_modes: HashMap::from([
                // Structured documents - search metadata
                ("pdf".to_string(), SearchMode::Metadata),
                ("docx".to_string(), SearchMode::Metadata),
                ("xlsx".to_string(), SearchMode::Metadata),
                ("pptx".to_string(), SearchMode::Metadata),
                ("odt".to_string(), SearchMode::Metadata),
                ("ods".to_string(), SearchMode::Metadata),
                ("odp".to_string(), SearchMode::Metadata),
                // Archives - search filenames
                ("zip".to_string(), SearchMode::Filename),
                ("tar".to_string(), SearchMode::Filename),
                ("gz".to_string(), SearchMode::Filename),
                ("bz2".to_string(), SearchMode::Filename),
                ("xz".to_string(), SearchMode::Filename),
                ("7z".to_string(), SearchMode::Filename),
                ("rar".to_string(), SearchMode::Filename),
                ("cab".to_string(), SearchMode::Filename),
                // Structured data - full structured search
                ("json".to_string(), SearchMode::Structured),
                ("xml".to_string(), SearchMode::Structured),
                ("yaml".to_string(), SearchMode::Structured),
                ("yml".to_string(), SearchMode::Structured),
                ("toml".to_string(), SearchMode::Structured),
                // Media files - search metadata
                ("mp3".to_string(), SearchMode::Metadata),
                ("flac".to_string(), SearchMode::Metadata),
                ("wav".to_string(), SearchMode::Metadata),
                ("mp4".to_string(), SearchMode::Metadata),
                ("mkv".to_string(), SearchMode::Metadata),
                ("jpg".to_string(), SearchMode::Metadata),
                ("png".to_string(), SearchMode::Metadata),
            ]),
        }
    }

    /// Determine if a file should be searched and how
    pub fn should_search(&self, path: &Path, metadata: &Metadata) -> SearchDecision {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        // Never search these
        if self.never_search.contains(&ext) {
            return SearchDecision::Skip(format!("Never search file type: {}", ext));
        }

        // Always search these
        if self.always_search.contains(&ext) {
            return self.check_size_limits(&ext, metadata, SearchMode::FullText);
        }

        // Conditional search
        if self.conditional_search.contains(&ext) {
            let search_mode = self
                .search_modes
                .get(&ext)
                .cloned()
                .unwrap_or(SearchMode::Metadata);
            return self.check_conditional_search(&ext, metadata, search_mode);
        }

        // Skip by default
        if self.skip_by_default.contains(&ext) {
            return SearchDecision::Skip(format!("Skip by default: {}", ext));
        }

        // Unknown extension - use MIME type detection
        self.classify_by_mime(path, metadata)
    }

    fn check_size_limits(
        &self,
        ext: &str,
        metadata: &Metadata,
        default_mode: SearchMode,
    ) -> SearchDecision {
        let file_size = metadata.len();
        let limit = self.size_limits.get(ext).unwrap_or(&(50 * 1024 * 1024));

        if file_size > *limit {
            SearchDecision::Skip(format!(
                "File too large: {} bytes > {} bytes (limit for .{})",
                file_size, limit, ext
            ))
        } else {
            SearchDecision::Search(default_mode)
        }
    }

    fn check_conditional_search(
        &self,
        ext: &str,
        metadata: &Metadata,
        search_mode: SearchMode,
    ) -> SearchDecision {
        let file_size = metadata.len();
        let limit = self.size_limits.get(ext).unwrap_or(&(10 * 1024 * 1024));

        if file_size > *limit {
            SearchDecision::Skip(format!(
                "Conditional file too large: {} bytes > {} bytes (limit for .{})",
                file_size, limit, ext
            ))
        } else {
            SearchDecision::Conditional(search_mode, format!("Conditional search for .{}", ext))
        }
    }

    fn classify_by_mime(&self, path: &Path, metadata: &Metadata) -> SearchDecision {
        // Use infer crate for MIME type detection
        if let Ok(Some(kind)) = infer::get_from_path(path) {
            let mime = kind.mime_type();
            return self.classify_by_mime_type(mime, metadata);
        }

        // Fallback: check if file is likely text by reading first few bytes
        if self.is_likely_text_file(path) {
            SearchDecision::Search(SearchMode::FullText)
        } else {
            SearchDecision::Skip("Unknown file type, not text-like".to_string())
        }
    }

    fn classify_by_mime_type(&self, mime: &str, metadata: &Metadata) -> SearchDecision {
        match mime {
            m if m.starts_with("text/") => SearchDecision::Search(SearchMode::FullText),
            m if m.starts_with("application/json") || m.starts_with("application/xml") => {
                SearchDecision::Search(SearchMode::Structured)
            }
            m if m.starts_with("application/pdf") => {
                self.check_conditional_search("pdf", metadata, SearchMode::Metadata)
            }
            m if m.starts_with("application/zip") || m.starts_with("application/x-tar") => {
                self.check_conditional_search("zip", metadata, SearchMode::Filename)
            }
            m if m.starts_with("image/") || m.starts_with("video/") || m.starts_with("audio/") => {
                self.check_conditional_search("media", metadata, SearchMode::Metadata)
            }
            m if m.starts_with("application/octet-stream") => {
                SearchDecision::Skip("Binary file detected".to_string())
            }
            _ => SearchDecision::Skip(format!("Unknown MIME type: {}", mime)),
        }
    }

    fn is_likely_text_file(&self, path: &Path) -> bool {
        if let Ok(content) = std::fs::read(path) {
            let sample_size = content.len().min(1024);
            if sample_size == 0 {
                return false;
            }

            let sample = &content[..sample_size];
            let null_bytes = sample.iter().filter(|&&b| b == 0).count();
            let text_ratio = (sample_size - null_bytes) as f64 / sample_size as f64;

            // Consider it text if less than 10% null bytes
            text_ratio > 0.9
        } else {
            false
        }
    }

    /// Get search mode for a specific file type
    pub fn get_search_mode(&self, ext: &str) -> Option<SearchMode> {
        self.search_modes.get(ext).cloned()
    }

    /// Check if a file type is always searched
    pub fn is_always_search(&self, ext: &str) -> bool {
        self.always_search.contains(ext)
    }

    /// Check if a file type is conditionally searched
    pub fn is_conditional_search(&self, ext: &str) -> bool {
        self.conditional_search.contains(ext)
    }

    /// Check if a file type is skipped by default
    pub fn is_skip_by_default(&self, ext: &str) -> bool {
        self.skip_by_default.contains(ext)
    }

    /// Check if a file type is never searched
    pub fn is_never_search(&self, ext: &str) -> bool {
        self.never_search.contains(ext)
    }

    /// Get size limit for a file type
    pub fn get_size_limit(&self, ext: &str) -> Option<u64> {
        self.size_limits.get(ext).cloned()
    }

    /// Add custom file type rules
    pub fn add_custom_rule(&mut self, ext: String, decision: SearchDecision) {
        match decision {
            SearchDecision::Search(mode) => {
                self.always_search.insert(ext.clone());
                self.search_modes.insert(ext, mode);
            }
            SearchDecision::Skip(_) => {
                self.never_search.insert(ext);
            }
            SearchDecision::Conditional(mode, _) => {
                self.conditional_search.insert(ext.clone());
                self.search_modes.insert(ext, mode);
            }
        }
    }
}

impl Default for FileTypeClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_always_search_files() {
        let classifier = FileTypeClassifier::new();
        let temp_dir = tempdir().unwrap();

        // Test text file
        let txt_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&txt_file).unwrap();
        file.write_all(b"test content").unwrap();
        let metadata = txt_file.metadata().unwrap();

        match classifier.should_search(&txt_file, &metadata) {
            SearchDecision::Search(SearchMode::FullText) => {}
            other => panic!("Expected Search(FullText), got {:?}", other),
        }
    }

    #[test]
    fn test_skip_by_default_files() {
        let classifier = FileTypeClassifier::new();
        let temp_dir = tempdir().unwrap();

        // Test executable file
        let exe_file = temp_dir.path().join("test.exe");
        File::create(&exe_file).unwrap();
        let metadata = exe_file.metadata().unwrap();

        match classifier.should_search(&exe_file, &metadata) {
            SearchDecision::Skip(reason) => {
                assert!(reason.contains("Skip by default"));
            }
            other => panic!("Expected Skip, got {:?}", other),
        }
    }

    #[test]
    fn test_conditional_search_files() {
        let classifier = FileTypeClassifier::new();
        let temp_dir = tempdir().unwrap();

        // Test PDF file
        let pdf_file = temp_dir.path().join("test.pdf");
        File::create(&pdf_file).unwrap();
        let metadata = pdf_file.metadata().unwrap();

        match classifier.should_search(&pdf_file, &metadata) {
            SearchDecision::Conditional(SearchMode::Metadata, reason) => {
                assert!(reason.contains("Conditional search"));
            }
            other => panic!("Expected Conditional(Metadata), got {:?}", other),
        }
    }

    #[test]
    fn test_size_limits() {
        let classifier = FileTypeClassifier::new();

        // Test size limit for text files
        let limit = classifier.get_size_limit("txt").unwrap();
        assert_eq!(limit, 100 * 1024 * 1024);

        // Test size limit for PDF files
        let limit = classifier.get_size_limit("pdf").unwrap();
        assert_eq!(limit, 10 * 1024 * 1024);
    }
}
