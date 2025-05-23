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
            if metadata.len() > max_size * 1024 * 1024 {
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
    println!("{}", header_separator);
    println!(
        "{:<60} {:>15} {:<10} {}",
        "Path".green().bold(),
        "Size".green().bold(),
        "Type".green().bold(),
        "Binary".green().bold()
    );
    println!("{}", header_separator);

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

// pub fn print_extension_stats(
//     extension_counts: &std::collections::HashMap<String, usize>,
//     total_size: u64,
// ) {
//     let mut ext_stats: Vec<_> = extension_counts.iter().collect();
//     ext_stats.par_sort_by(|a, b| b.1.cmp(a.1));

//     let separator = "-".repeat(50).dimmed();
//     println!("\n{}", separator);
//     println!("{}", "Summary:".bold());

//     let total_files: usize = extension_counts.values().sum();
//     println!(
//         "{}: {}",
//         "Total files".bold(),
//         total_files.to_string().replace(",", "_")
//     );
//     println!("{}: {}", "Total size".bold(), format_size(total_size));

//     println!("\n{}", separator);
//     println!("{}:", "Extensions".bold());

//     let max_ext_len = ext_stats
//         .par_iter()
//         .map(|(ext, _)| ext.len())
//         .max()
//         .unwrap_or(0);

//     ext_stats.par_iter().for_each(|(ext, count)| {
//         let ext_display = if ext.is_empty() {
//             "(no extension)"
//         } else {
//             ext
//         };
//         println!(
//             "  {:<width$} {:>5} files",
//             format!(".{}", ext_display).yellow(),
//             count.to_string().cyan(),
//             width = max_ext_len + 1
//         );
//     });
// }
