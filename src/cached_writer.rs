use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::PathBuf;

const OUTPUT_FLUSH_THRESHOLD: usize = 2 * 1024 * 1024;
const MAX_PARSED_LOGLINE_LENGTH: usize = 2 * 1024;

pub struct CachedWriter {
    cache: HashMap<String, String>,
    folder: String,
    active_files: HashSet<String>,
}

impl CachedWriter {
    
    pub fn new(folder: &str) -> CachedWriter {
        CachedWriter {
            cache: HashMap::new(),
            folder: folder.to_string(),
            active_files: HashSet::new(),
        }
    }

    pub fn write(&mut self, token: &str, content: Box<String>) -> io::Result<()> {
        if !self.cache.contains_key(token) {
            self.cache.insert(token.to_string(),
             String::with_capacity(OUTPUT_FLUSH_THRESHOLD + MAX_PARSED_LOGLINE_LENGTH));
        }

        let cache = self.cache.get_mut(token).unwrap();
        cache.push_str(&content);

        if cache.len() > OUTPUT_FLUSH_THRESHOLD {
            self.do_write(token)?;
        }
        Ok(())
    }

    fn do_write(&mut self, token: &str) -> io::Result<()> {
        let mut filepath = PathBuf::from(&self.folder);
        filepath.push(token.to_string() + ".txt");

        let mut f = match self.active_files.contains(token) {
            true => OpenOptions::new().write(true).append(true).open(filepath)?,
            false => {
                self.active_files.insert(token.to_string());
                OpenOptions::new().write(true).create(true).truncate(true).open(filepath)?
            },
        };

        let cache = self.cache.get_mut(token).unwrap();
        f.write(cache.as_bytes())?;

        cache.clear();
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        // TODO: any better way to get keys without keeping an active reference?
        let mut tokens: Vec<String> = Vec::new();
        for (k, _) in &self.cache {
            tokens.push(k.clone());
        }

        for token in tokens {
            self.do_write(&token)?;
        }
        Ok(())
    }
}
