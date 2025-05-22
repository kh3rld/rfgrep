use anyhow::Result;
use regex::Regex;
use std::fs;
use std::io::Read;

pub fn is_binary(file: &std::path::Path) -> bool {
    if let Ok(mut f) = fs::File::open(file) {
        let mut buffer = [0u8; 1024];
        if let Ok(bytes_read) = f.read(&mut buffer) {
            if bytes_read > 0 {
                return buffer[..bytes_read].contains(&0u8);
            }
        }
    }
    false
}

pub fn search_file(path: &std::path::Path, pattern: &Regex) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(pattern
        .find_iter(&content)
        .map(|m| format!("{}: {}", path.display(), m.as_str()))
        .collect())
}
