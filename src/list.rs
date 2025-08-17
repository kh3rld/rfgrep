use byte_unit::Byte;
use colored::*;
use rayon::prelude::*;
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
            if !exts.iter().any(|e| e.eq_ignore_ascii_case(file_ext)) {
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
            if metadata.len() > (max_size as u64) * 1024 * 1024 {
                return false;
            }
        }
    }

    true
}

fn format_size(size: u64) -> String {
    use byte_unit::UnitType;
    let bytes = Byte::from_u64(size);
    bytes.get_appropriate_unit(UnitType::Binary).to_string()
}

pub fn print_simple_list(files: &[FileInfo]) {
    files.par_iter().for_each(|file| {
        println!("{}", file.path.display().to_string().cyan());
    });
}

pub fn print_long_format(files: &[FileInfo]) {
    let header_separator = "-".repeat(100).dimmed();
    println!("{header_separator}");
    println!(
        "{:<60} {:>15} {:<10} {}",
        "Path".green().bold(),
        "Size".green().bold(),
        "Type".green().bold(),
        "Binary".green().bold()
    );
    println!("{header_separator}");

    files.par_iter().for_each(|file| {
        let size_str = format_size(file.size);
        let binary_indicator = if file.is_binary {
            "Yes".yellow()
        } else {
            "No".green()
        };

        let path_str = if file.path.to_string_lossy().len() > 57 {
            format!(
                "...{}",
                file.path.file_name().unwrap_or_default().to_string_lossy()
            )
        } else {
            file.path.display().to_string()
        };

        println!(
            "{:<60} {:>15} {:<10} {}",
            path_str.cyan(),
            size_str,
            format!(".{}", file.extension).blue(),
            binary_indicator
        );
    });
}
