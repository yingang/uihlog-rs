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
                        if let Some(_) = Self::extract_id(&path) {
                            files.push(path); 
                        } else {
                            println!("invalid file name, skipped: {:?}", &path);
                        }
                    }
                }
            }
        }
        files.sort_by(|a, b| {
            let id_a = Self::extract_id(a).unwrap();    // safe to use unwrap here since it has been tested
            let id_b = Self::extract_id(b).unwrap();
            id_a.cmp(&id_b)
            }
        );
        SortedFileList { files: files, index: 0 }
    }

    fn extract_id(path: &PathBuf) -> Option<i32> {
        if let Some(filename) = path.file_stem().and_then(|f|f.to_str()) {
            if let Some(idx) = filename.find('.') {
                if let Ok(id) = filename[..idx].parse::<i32>() {
                    return Some(id)
                }
            }
        }
        None
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