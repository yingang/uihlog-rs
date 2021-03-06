use crate::file_system::get_file_list;

use std::path::{Path, PathBuf};

pub struct SortedFileList {
    files: Vec<PathBuf>,
    index: usize,
}

impl SortedFileList {
    pub fn new(folder: &Path) -> SortedFileList {
        let files = get_file_list(folder);
        Self::do_new(files)
    }

    fn do_new(files: Vec<PathBuf>) -> SortedFileList {
        SortedFileList { files: Self::sort_files(files), index:0 }
    }

    fn sort_files(files: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut sorted: Vec<PathBuf> = Vec::new();
        for file in files {
            if let Some(ext) = file.extension() {
                if ext != "uihlog" {
                    continue;
                }
                if Self::extract_id(&file) == None {
                    println!("invalid file name, skipped: {:?}", &file);
                    continue;
                }
                sorted.push(file);
            }
        }
        sorted.sort_by(|a, b| {
            let id_a = Self::extract_id(a).unwrap();    // safe to use unwrap here since it has been tested
            let id_b = Self::extract_id(b).unwrap();
            id_a.cmp(&id_b)
            }
        );
        sorted
    }

    fn extract_id(path: &PathBuf) -> Option<i32> {
        if let Some(filename) = path.file_stem().and_then(|f|f.to_str()) {
            let id = match filename.find(".") {
                Some(idx) => &filename[..idx],
                None => &filename
            };
            if let Ok(id) = id.parse::<i32>() {
                return Some(id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_extraction() {
        let path = PathBuf::from(r"c:\test\1.uihlog");
        assert_eq!(SortedFileList::extract_id(&path), Some(1));

        let path = PathBuf::from(r"c:\test\1.svc.uihlog");
        assert_eq!(SortedFileList::extract_id(&path), Some(1));
    }

    #[test]
    fn filenames_with_same_length() {
        let files = vec![PathBuf::from(r"c:\2.uihlog"), PathBuf::from(r"c:\1.uihlog")];
        let mut sfl = SortedFileList::do_new(files);
        assert!(sfl.count() == 2);
        assert!(sfl.next().unwrap().to_str().unwrap() == r"c:\1.uihlog");
        assert!(sfl.next().unwrap().to_str().unwrap() == r"c:\2.uihlog");
        assert!(sfl.next().is_none());
    }

    #[test]
    fn filenames_with_different_length() {
        let files = vec![PathBuf::from(r"c:\10.uihlog"), PathBuf::from(r"c:\2.uihlog")];
        let mut sfl = SortedFileList::do_new(files);
        assert!(sfl.count() == 2);
        assert!(sfl.next().unwrap().to_str().unwrap() == r"c:\2.uihlog");
        assert!(sfl.next().unwrap().to_str().unwrap() == r"c:\10.uihlog");
        assert!(sfl.next().is_none());
    }

    #[test]
    fn empty_folder() {
        let files: Vec<PathBuf> = Vec::new();
        let mut sfl = SortedFileList::do_new(files);
        assert!(sfl.count() == 0);
        assert!(sfl.next().is_none());
    }

    #[test]
    fn invalid_files() {
        let files= vec![PathBuf::from(r"c:\thumbs.db"), PathBuf::from(".cargo-lock"), PathBuf::from(r"c:\invalid.uihlog")];
        let mut sfl = SortedFileList::do_new(files);
        assert!(sfl.count() == 0);
        assert!(sfl.next().is_none());
    }
}