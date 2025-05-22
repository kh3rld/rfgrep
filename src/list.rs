use std::path::Path;

use crate::{cli::Cli, processor};

pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub extension: String,
    pub is_binary: bool,
}

pub fn should_list_file(path: &Path, cli: &Cli, extensions: &Option<Vec<String>>) -> bool {
    if let Some(exts) = extensions {
        if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
            if !exts.iter().any(|e| e == file_ext) {
                return false;
            }
        } else {
            return false;
        }
    }

    if cli.skip_binary && processor::is_binary(path) {
        return false;
    }

    if let Some(max_size) = cli.max_size {
        if let Ok(metadata) = path.metadata() {
            if metadata.len() > max_size * 1024 * 1024 {
                return false;
            }
        }
    }

    true
}

pub fn print_simple_list(files: &[FileInfo]) {
    for file in files {
        println!("{}", file.path.display());
    }
}

pub fn print_long_format(files: &[FileInfo]) {
    for file in files {
        println!(
            "{:<60} {:>8} KB {:<10} {}",
            file.path.display(),
            file.size / 1024,
            format!(".{}", file.extension),
            if file.is_binary { "(binary)" } else { "" }
        );
    }
}
