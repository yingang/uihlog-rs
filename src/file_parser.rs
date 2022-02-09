use crate::file_system::read_file;
use crate::log_parser::LogParser;

use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

pub fn parse_file(filepath: &Path) -> io::Result<()> {
    if let Some(content) = read_file(&filepath) {
        let output = PathBuf::from(filepath.to_str().unwrap().to_string() + ".txt");
        let mut f = BufWriter::new(File::create(&output)?);
        let mut parser = LogParser::new();
        for line in &parser.parse_sync(content) {
            f.write(&line.content.as_bytes())?;
        }
        f.flush()?;
    }
    Ok(())
}
