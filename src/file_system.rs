use crate::buffered_output::FileWriter;

use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn get_file_list(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = fs::read_dir(folder) {
        for path in dir {
            if let Ok(path) = path {
                let path = path.path();
                if path.is_file() {
                    files.push(path); 
                }
            }
        }
    }
    files
}

pub fn read_file(filepath: &Path) -> Option<String> {
    if let Ok(data) = fs::read(filepath) {
        let data = String::from_utf8_lossy(&data);  // consider log file with invalid UTF8 content
        return Some(data.into_owned())
    }
    println!("failed to read from file {:?}", &filepath);
    None
}

pub struct RealFileWriter {
}

impl RealFileWriter {
    pub fn new() -> Self {
        RealFileWriter {}
    }
}

impl FileWriter for RealFileWriter {
    fn write(&self, filepath: &Path, content: &String, append: bool) -> io::Result<()> {
        let mut f = match append {
            true => OpenOptions::new().append(true).open(filepath)?,
            false => OpenOptions::new().write(true).truncate(true).open(filepath)?,
        };
    
        f.write(&content.as_bytes())?;
        Ok(())
    }
}