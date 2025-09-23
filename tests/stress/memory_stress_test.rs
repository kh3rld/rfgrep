use std::fs;
use std::path::Path;
use tempfile::TempDir;
use rfgrep::processor::search_file;
use regex::Regex;

#[test]
fn test_memory_usage_large_files() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.txt");
    
    let mut content = String::with_capacity(10 * 1024 * 1024);
    for i in 0..100000 {
        writeln!(content, "Line {} with some content and pattern", i).unwrap();
    }
    
    fs::write(&large_file, content).unwrap();
    
    let pattern = Regex::new("pattern").unwrap();
    let matches = search_file(&large_file, &pattern).unwrap();
    
    assert_eq!(matches.len(), 100000);
}

#[test]
fn test_memory_usage_many_files() {
    let temp_dir = TempDir::new().unwrap();
    
    for i in 0..1000 {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        fs::write(&file_path, format!("File {} with pattern content", i)).unwrap();
    }
    
    let pattern = Regex::new("pattern").unwrap();
    let mut total_matches = 0;
    
    for i in 0..1000 {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        let matches = search_file(&file_path, &pattern).unwrap();
        total_matches += matches.len();
    }
    
    assert_eq!(total_matches, 1000);
}

#[test]
fn test_memory_usage_mixed_sizes() {
    let temp_dir = TempDir::new().unwrap();
    
    let sizes = vec![1024, 10240, 102400, 1024000]; 
    
    for (i, size) in sizes.iter().enumerate() {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        let mut content = String::with_capacity(*size);
        
        for j in 0..(*size / 50) { 
            writeln!(content, "Line {} in file {} with pattern", j, i).unwrap();
        }
        
        fs::write(&file_path, content).unwrap();
    }
    
    let pattern = Regex::new("pattern").unwrap();
    let mut total_matches = 0;
    
    for i in 0..sizes.len() {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        let matches = search_file(&file_path, &pattern).unwrap();
        total_matches += matches.len();
    }
    
    assert!(total_matches > 0);
}

#[test]
fn test_memory_usage_unicode_content() {
    let temp_dir = TempDir::new().unwrap();
    let unicode_file = temp_dir.path().join("unicode.txt");
    
    let mut content = String::new();
    content.push_str("English text with pattern\n");
    content.push_str("中文文本包含模式\n");
    content.push_str("العربية مع النمط\n");
    content.push_str("Русский текст с шаблоном\n");
    content.push_str("日本語のテキストにパターン\n");
    content.push_str("한국어 텍스트에 패턴\n");
    
    for _ in 0..1000 {
        content.push_str(&content.clone());
    }
    
    fs::write(&unicode_file, content).unwrap();
    
    let pattern = Regex::new("pattern").unwrap();
    let matches = search_file(&unicode_file, &pattern).unwrap();
    
    assert!(matches.len() > 0);
}

#[test]
fn test_memory_usage_binary_detection() {
    let temp_dir = TempDir::new().unwrap();
    
    let binary_file = temp_dir.path().join("binary.bin");
    let mut binary_content = vec![0u8; 1024 * 1024]; 
    binary_content[100] = 1; 
    fs::write(&binary_file, binary_content).unwrap();
    
    let text_file = temp_dir.path().join("text.txt");
    let text_content = "pattern ".repeat(1024 * 1024 / 8); 
    fs::write(&text_file, text_content).unwrap();
    
    let pattern = Regex::new("pattern").unwrap();
    
    let binary_matches = search_file(&binary_file, &pattern).unwrap();
    assert_eq!(binary_matches.len(), 0);
    
    let text_matches = search_file(&text_file, &pattern).unwrap();
    assert!(text_matches.len() > 0);
}

#[test]
fn test_memory_usage_regex_complexity() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    
    let mut content = String::new();
    for i in 0..10000 {
        writeln!(content, "Email: user{}@example.com, Phone: (555) {}-{:04}", 
                i, i % 1000, i % 10000).unwrap();
    }
    
    fs::write(&test_file, content).unwrap();
    
    let patterns = vec![
        (r"\b\w+@\w+\.\w+\b", "email"),
        (r"\(\d{3}\) \d{3}-\d{4}", "phone"),
        (r"\b\w+\b", "word"),
    ];
    
    for (pattern_str, _name) in patterns {
        let pattern = Regex::new(pattern_str).unwrap();
        let matches = search_file(&test_file, &pattern).unwrap();
        assert!(matches.len() > 0, "Pattern {} should find matches", pattern_str);
    }
}
