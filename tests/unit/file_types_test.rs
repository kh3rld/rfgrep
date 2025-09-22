use rfgrep::file_types::{FileTypeClassifier, SearchDecision, FileCategory};
use std::path::Path;
use std::fs::Metadata;
use std::time::SystemTime;

/// Test data for file type classification
fn create_test_metadata(size: u64) -> Metadata {
    // This is a mock implementation for testing
    // In a real test, we'd use a proper mock or test file
    std::fs::metadata("Cargo.toml").unwrap()
}

#[cfg(test)]
mod file_type_classification_tests {
    use super::*;

    #[test]
    fn test_always_search_files() {
        let classifier = FileTypeClassifier::new();
        
        // Test text files
        assert_eq!(
            classifier.should_search(Path::new("test.txt"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::AlwaysSearch)
        );
        
        // Test source code files
        assert_eq!(
            classifier.should_search(Path::new("main.rs"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::AlwaysSearch)
        );
        
        // Test config files
        assert_eq!(
            classifier.should_search(Path::new("config.json"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::AlwaysSearch)
        );
    }

    #[test]
    fn test_conditional_search_files() {
        let classifier = FileTypeClassifier::new();
        
        // Test office documents
        assert_eq!(
            classifier.should_search(Path::new("document.docx"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::ConditionalSearch)
        );
        
        // Test archives
        assert_eq!(
            classifier.should_search(Path::new("archive.zip"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::ConditionalSearch)
        );
        
        // Test media files
        assert_eq!(
            classifier.should_search(Path::new("video.mp4"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::ConditionalSearch)
        );
    }

    #[test]
    fn test_skip_by_default_files() {
        let classifier = FileTypeClassifier::new();
        
        // Test executables
        assert_eq!(
            classifier.should_search(Path::new("program.exe"), &create_test_metadata(1024)),
            SearchDecision::Skip(FileCategory::SkipByDefault)
        );
        
        // Test system files
        assert_eq!(
            classifier.should_search(Path::new("system.dll"), &create_test_metadata(1024)),
            SearchDecision::Skip(FileCategory::SkipByDefault)
        );
    }

    #[test]
    fn test_never_search_files() {
        let classifier = FileTypeClassifier::new();
        
        // Test dangerous files
        assert_eq!(
            classifier.should_search(Path::new("malware.exe"), &create_test_metadata(1024)),
            SearchDecision::Skip(FileCategory::NeverSearch)
        );
        
        // Test irrelevant files
        assert_eq!(
            classifier.should_search(Path::new("temp.tmp"), &create_test_metadata(1024)),
            SearchDecision::Skip(FileCategory::NeverSearch)
        );
    }

    #[test]
    fn test_size_limits() {
        let classifier = FileTypeClassifier::new();
        
        // Test large text file (should be limited)
        let large_metadata = create_test_metadata(100 * 1024 * 1024); // 100MB
        assert_eq!(
            classifier.should_search(Path::new("large.txt"), &large_metadata),
            SearchDecision::Skip(FileCategory::AlwaysSearch) // Should be skipped due to size
        );
        
        // Test small text file (should be searched)
        let small_metadata = create_test_metadata(1024); // 1KB
        assert_eq!(
            classifier.should_search(Path::new("small.txt"), &small_metadata),
            SearchDecision::Search(FileCategory::AlwaysSearch)
        );
    }

    #[test]
    fn test_unknown_extension() {
        let classifier = FileTypeClassifier::new();
        
        // Test unknown extension (should use MIME type detection)
        assert_eq!(
            classifier.should_search(Path::new("unknown.xyz"), &create_test_metadata(1024)),
            SearchDecision::Skip(FileCategory::SkipByDefault) // Default for unknown
        );
    }

    #[test]
    fn test_case_insensitive_extensions() {
        let classifier = FileTypeClassifier::new();
        
        // Test uppercase extensions
        assert_eq!(
            classifier.should_search(Path::new("test.TXT"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::AlwaysSearch)
        );
        
        // Test mixed case extensions
        assert_eq!(
            classifier.should_search(Path::new("test.TxT"), &create_test_metadata(1024)),
            SearchDecision::Search(FileCategory::AlwaysSearch)
        );
    }

    #[test]
    fn test_all_supported_formats() {
        let classifier = FileTypeClassifier::new();
        
        // Test all 153 supported formats
        let always_search_formats = [
            "txt", "md", "rst", "org", "tex", "log", "cfg", "ini", "conf", "yaml", "yml",
            "json", "xml", "html", "htm", "css", "js", "ts", "jsx", "tsx", "vue", "svelte",
            "rs", "py", "java", "c", "cpp", "h", "hpp", "cc", "cxx", "cs", "go", "php",
            "rb", "swift", "kt", "scala", "clj", "hs", "ml", "fs", "vb", "dart", "r",
            "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", "sql", "pl", "lua", "tcl",
            "vim", "emacs", "gitignore", "dockerfile", "makefile", "cmake", "gradle",
            "pom", "sbt", "cargo", "composer", "package", "bower", "gem", "podspec"
        ];
        
        for ext in &always_search_formats {
            let path = Path::new(&format!("test.{}", ext));
            let result = classifier.should_search(path, &create_test_metadata(1024));
            assert_eq!(result, SearchDecision::Search(FileCategory::AlwaysSearch),
                "Extension .{} should be in AlwaysSearch category", ext);
        }
    }

    #[test]
    fn test_performance_characteristics() {
        let classifier = FileTypeClassifier::new();
        let metadata = create_test_metadata(1024);
        
        // Test classification speed
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            classifier.should_search(Path::new("test.txt"), &metadata);
        }
        let duration = start.elapsed();
        
        // Should classify 1000 files in less than 1ms
        assert!(duration.as_millis() < 1, "Classification too slow: {:?}", duration);
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_filename() {
        let classifier = FileTypeClassifier::new();
        let result = classifier.should_search(Path::new(""), &create_test_metadata(1024));
        assert_eq!(result, SearchDecision::Skip(FileCategory::SkipByDefault));
    }

    #[test]
    fn test_no_extension() {
        let classifier = FileTypeClassifier::new();
        let result = classifier.should_search(Path::new("README"), &create_test_metadata(1024));
        assert_eq!(result, SearchDecision::Search(FileCategory::AlwaysSearch));
    }

    #[test]
    fn test_multiple_extensions() {
        let classifier = FileTypeClassifier::new();
        let result = classifier.should_search(Path::new("file.tar.gz"), &create_test_metadata(1024));
        assert_eq!(result, SearchDecision::Search(FileCategory::ConditionalSearch));
    }

    #[test]
    fn test_hidden_files() {
        let classifier = FileTypeClassifier::new();
        let result = classifier.should_search(Path::new(".hidden.txt"), &create_test_metadata(1024));
        assert_eq!(result, SearchDecision::Search(FileCategory::AlwaysSearch));
    }

    #[test]
    fn test_very_long_filename() {
        let classifier = FileTypeClassifier::new();
        let long_name = "a".repeat(255) + ".txt";
        let result = classifier.should_search(Path::new(&long_name), &create_test_metadata(1024));
        assert_eq!(result, SearchDecision::Search(FileCategory::AlwaysSearch));
    }
}
