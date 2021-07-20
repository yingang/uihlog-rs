use std::fs;
use std::path::{Path, PathBuf};

pub struct SortedFileList {
    files: Vec<PathBuf>,
    index: usize,
}

impl SortedFileList {
    pub fn new(folder: &Path) -> SortedFileList {
        let mut files: Vec<PathBuf> = Vec::new();
        if let Ok(dir) = fs::read_dir(folder) {
            for path in dir {
                if let Ok(path) = path {
                    let path = path.path();
                    if path.is_file() && path.extension().unwrap() == "uihlog" {
                        files.push(path); 
                    }
                }
            }
        }
        files.sort_by(|a, b| {
            let idx_a = a.file_stem().unwrap().to_str().unwrap();
            let idx_b = b.file_stem().unwrap().to_str().unwrap();
            idx_a.parse::<i32>().unwrap().cmp(&idx_b.parse::<i32>().unwrap())
            }
        );
        SortedFileList { files: files, index: 0 }
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        if self.index < self.files.len() {
            self.index += 1;
            Some(self.files[self.index - 1].clone())
        } else {
            None
        }
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }
}