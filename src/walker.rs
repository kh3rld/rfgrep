use walkdir::{WalkDir, DirEntry};
use std::path::Path;

const IGNORED_DIRS: [&str; 4] = [".git", "node_modules", ".idea", "target"];

pub fn walk_dir(path: &Path, recursive: bool, show_hidden: bool) -> impl Iterator<Item = DirEntry> {
    let walker = WalkDir::new(path)
        .into_iter()
        .filter_entry(move |e| {
            let is_hidden = e.file_name().to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false);
            
            let ignore_status = if show_hidden {
                !IGNORED_DIRS.contains(&e.file_name().to_str().unwrap_or(""))
            } else {
                !is_hidden && !IGNORED_DIRS.contains(&e.file_name().to_str().unwrap_or(""))
            };

            if recursive {
                ignore_status
            } else {
                e.depth() == 0 || ignore_status
            }
        })
        .filter_map(|e| e.ok());

    walker
}
