//! Mock file system for testing
use std::collections::HashMap;
use std::path::PathBuf;

/// Mock file system for testing
pub struct MockFileSystem {
    files: HashMap<PathBuf, Vec<u8>>,
    directories: HashMap<PathBuf, Vec<PathBuf>>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            directories: HashMap::new(),
        }
    }

    pub fn create_file(&mut self, path: PathBuf, content: Vec<u8>) {
        self.files.insert(path, content);
    }

    pub fn create_dir(&mut self, path: PathBuf) {
        self.directories.insert(path, Vec::new());
    }

    pub fn read_file(&self, path: &PathBuf) -> Option<&Vec<u8>> {
        self.files.get(path)
    }

    pub fn file_exists(&self, path: &PathBuf) -> bool {
        self.files.contains_key(path)
    }

    pub fn dir_exists(&self, path: &PathBuf) -> bool {
        self.directories.contains_key(path)
    }
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}
