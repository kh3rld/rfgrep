use ignore::{DirEntry, WalkBuilder};
use std::path::Path;

pub fn walk_dir(path: &Path, recursive: bool, show_hidden: bool) -> impl Iterator<Item = DirEntry> {
    let max_depth = if recursive { None } else { Some(1) };
    WalkBuilder::new(path)
        .hidden(!show_hidden)
        .git_global(!show_hidden)
        .git_ignore(!show_hidden)
        .git_exclude(!show_hidden)
        .ignore(!show_hidden)
        .max_depth(max_depth)
        .build()
        .filter_map(Result::ok)
}
