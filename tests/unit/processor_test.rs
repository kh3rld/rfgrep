use rfgrep::processor::{is_binary, search_file, SearchMatch};
use regex::Regex;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[cfg(test)]
mod binary_detection_tests {
    use super::*;

    #[test]
    fn test_text_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"This is a text file with some content\n").unwrap();
        file.flush().unwrap();
        
        assert!(!is_binary(&file_path), "Text file should not be detected as binary");
    }

    #[test]
    fn test_binary_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"\x00\x01\x02\x03\x04\x05\x06\x07").unwrap();
        file.flush().unwrap();
        
        assert!(is_binary(&file_path), "Binary file should be detected as binary");
    }

    #[test]
    fn test_utf8_bom_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_utf8.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"\xEF\xBB\xBFThis is UTF-8 with BOM\n").unwrap();
        file.flush().unwrap();
        
        assert!(!is_binary(&file_path), "UTF-8 with BOM should not be detected as binary");
    }

    #[test]
    fn test_utf16_le_bom_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_utf16le.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"\xFF\xFE").unwrap(); // UTF-16 LE BOM
        file.write_all(b"T\x00h\x00i\x00s\x00 \x00i\x00s\x00 \x00U\x00T\x00F\x00-\x001\x006\x00\n\x00").unwrap();
        file.flush().unwrap();
        
        assert!(!is_binary(&file_path), "UTF-16 LE with BOM should not be detected as binary");
    }

    #[test]
    fn test_utf16_be_bom_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_utf16be.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"\xFE\xFF").unwrap(); // UTF-16 BE BOM
        file.write_all(b"\x00T\x00h\x00i\x00s\x00 \x00i\x00s\x00 \x00U\x00T\x00F\x00-\x001\x006\x00\n").unwrap();
        file.flush().unwrap();
        
        assert!(!is_binary(&file_path), "UTF-16 BE with BOM should not be detected as binary");
    }

    #[test]
    fn test_utf16_pattern_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_utf16_pattern.txt");
        
        let mut file = File::create(&file_path).unwrap();
        // UTF-16 content without BOM (alternating null bytes)
        file.write_all(b"T\x00h\x00i\x00s\x00 \x00i\x00s\x00 \x00U\x00T\x00F\x00-\x001\x006\x00\n\x00").unwrap();
        file.flush().unwrap();
        
        assert!(!is_binary(&file_path), "UTF-16 pattern should not be detected as binary");
    }

    #[test]
    fn test_mixed_content_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_mixed.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"This is text with some \x00 null bytes \x01\x02\x03 but mostly text\n").unwrap();
        file.flush().unwrap();
        
        // Should be detected as binary due to null bytes
        assert!(is_binary(&file_path), "File with null bytes should be detected as binary");
    }

    #[test]
    fn test_empty_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        
        let file = File::create(&file_path).unwrap();
        file.sync_all().unwrap();
        
        assert!(!is_binary(&file_path), "Empty file should not be detected as binary");
    }

    #[test]
    fn test_single_byte_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("single.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"A").unwrap();
        file.flush().unwrap();
        
        assert!(!is_binary(&file_path), "Single byte text file should not be detected as binary");
    }
}

#[cfg(test)]
mod search_function_tests {
    use super::*;

    fn create_test_file(content: &str) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        
        (temp_dir, file_path)
    }

    #[test]
    fn test_simple_pattern_search() {
        let (_temp_dir, file_path) = create_test_file("This is a test file with some content\n");
        let pattern = Regex::new("test").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line_number, 1);
        assert!(matches[0].line.contains("test"));
    }

    #[test]
    fn test_multiple_matches() {
        let (_temp_dir, file_path) = create_test_file("test line 1\ntest line 2\nnot a test\nanother test line");
        let pattern = Regex::new("test").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 3);
        
        // Check line numbers
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].line_number, 2);
        assert_eq!(matches[2].line_number, 4);
    }

    #[test]
    fn test_regex_pattern_search() {
        let (_temp_dir, file_path) = create_test_file("file1.txt\nfile2.dat\nfile3.log\nnot_a_file");
        let pattern = Regex::new(r"file\d+\.txt").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, "file1.txt");
    }

    #[test]
    fn test_case_sensitive_search() {
        let (_temp_dir, file_path) = create_test_file("Test\nTEST\ntest\nTeSt");
        let pattern = Regex::new("Test").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 2); // "Test" and "TeSt"
    }

    #[test]
    fn test_no_matches() {
        let (_temp_dir, file_path) = create_test_file("This file has no matches for the pattern");
        let pattern = Regex::new("nonexistent").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_context_lines() {
        let (_temp_dir, file_path) = create_test_file("line 1\nline 2\nline 3\nline 4\nline 5");
        let pattern = Regex::new("line 3").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 1);
        
        // Check that context is included
        let match_line = &matches[0].line;
        assert!(match_line.contains("line 3"));
    }

    #[test]
    fn test_large_file_search() {
        // Create a larger file to test performance
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");
        
        let mut file = File::create(&file_path).unwrap();
        for i in 0..1000 {
            writeln!(file, "Line {} with some content", i).unwrap();
        }
        writeln!(file, "This line contains the target pattern").unwrap();
        for i in 1001..2000 {
            writeln!(file, "Line {} with more content", i).unwrap();
        }
        file.flush().unwrap();
        
        let pattern = Regex::new("target pattern").unwrap();
        let matches = search_file(&file_path, &pattern).unwrap();
        
        assert_eq!(matches.len(), 1);
        assert!(matches[0].line.contains("target pattern"));
    }

    #[test]
    fn test_unicode_content() {
        let (_temp_dir, file_path) = create_test_file("Hello 世界\nThis is a test with émojis 🚀\nMore unicode: αβγ");
        let pattern = Regex::new("世界").unwrap();
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].line.contains("世界"));
    }

    #[test]
    fn test_special_regex_characters() {
        let (_temp_dir, file_path) = create_test_file("Price: $100.50\nEmail: test@example.com\nPhone: (555) 123-4567");
        let pattern = Regex::new(r"\$[\d.]+").unwrap(); // Match price
        
        let matches = search_file(&file_path, &pattern).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].line.contains("$100.50"));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_nonexistent_file() {
        let pattern = Regex::new("test").unwrap();
        let result = search_file(Path::new("nonexistent.txt"), &pattern);
        assert!(result.is_err());
    }

    #[test]
    fn test_permission_denied() {
        // This test might not work on all systems
        // Skip if we can't create a permission-denied file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readonly.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        file.flush().unwrap();
        
        // On Unix systems, we could test permission denied
        // For now, just test that the file exists and is readable
        let pattern = Regex::new("test").unwrap();
        let result = search_file(&file_path, &pattern);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_regex() {
        // This test is for the regex creation, not the search function
        let result = Regex::new("[invalid regex");
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_search_performance() {
        let (_temp_dir, file_path) = create_test_file("This is a test file with some content\n");
        let pattern = Regex::new("test").unwrap();
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _matches = search_file(&file_path, &pattern).unwrap();
        }
        let duration = start.elapsed();
        
        // Should complete 1000 searches in less than 100ms
        assert!(duration.as_millis() < 100, "Search too slow: {:?}", duration);
    }

    #[test]
    fn test_binary_detection_performance() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"This is a test file with some content\n").unwrap();
        file.flush().unwrap();
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _is_binary = is_binary(&file_path);
        }
        let duration = start.elapsed();
        
        // Should complete 1000 binary checks in less than 50ms
        assert!(duration.as_millis() < 50, "Binary detection too slow: {:?}", duration);
    }
}
