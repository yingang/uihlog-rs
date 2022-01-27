use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

const OUTPUT_FLUSH_THRESHOLD: usize = 2 * 1024 * 1024;
const MAX_PARSED_LOGLINE_LENGTH: usize = 2 * 1024;

pub trait FileWriter {
    fn write(&self, filepath: &Path, content: &String, append: bool) -> io::Result<()>;
}

pub struct CachedWriter<'a, T: FileWriter> {
    cache: HashMap<String, String>,
    folder: String,
    active_files: HashSet<String>,
    writer: &'a T,
}

impl<'a, T> CachedWriter<'a, T>
where T: FileWriter,
{
    pub fn new(folder: &str, writer: &'a T) -> Self {
        Self {
            cache: HashMap::new(),
            folder: folder.to_string(),
            active_files: HashSet::new(),
            writer: &writer,
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
        let cache = self.cache.get_mut(token).unwrap();
        let append = self.active_files.contains(token);

        self.writer.write(&filepath, &cache, append)?;
        if append == false {
            self.active_files.insert(token.to_string());
        }

        cache.clear();
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let tokens: Vec<String> = self.cache.keys().cloned().collect();
        for token in tokens {
            self.do_write(token.as_str())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct FileState {
        pub length: usize,
        pub written_times: usize,
    }

    impl FileState {
        pub fn new() -> Self {
            FileState {
                length: 0, written_times: 0,
            }
        }
    }

    struct MockFileWriter {
        state: RefCell<HashMap<String, FileState>>,
    }

    impl FileWriter for MockFileWriter {
        fn write(&self, filepath: &Path, content: &String, append: bool) -> io::Result<()> {
            let filepath = filepath.to_str().unwrap().to_string();
            if self.state.borrow().contains_key(&filepath) {
                assert!(append == true);
            } else {
                assert!(append == false);
                self.state.borrow_mut().insert(filepath.to_string(), FileState::new());
            }

            let mut state = self.state.borrow_mut();
            let fs = state.get_mut(&filepath).unwrap();
            fs.length += content.len();
            fs.written_times += 1;
            Ok(())
        }
    }

    impl MockFileWriter {
        pub fn new() -> Self {
            MockFileWriter {
                state: RefCell::new(HashMap::new())
            }
        }

        pub fn file_exists(&self, filepath: String) -> bool {
            self.state.borrow().contains_key(&filepath)
        }

        pub fn get_file_length(&self, filepath: String) -> usize {
            assert!(self.state.borrow().contains_key(&filepath));
            self.state.borrow().get(&filepath).unwrap().length
        }

        pub fn get_file_written_times(&self, filepath: String) -> usize {
            assert!(self.state.borrow().contains_key(&filepath));
            self.state.borrow().get(&filepath).unwrap().written_times
        }
    }

    #[test]
    fn it_works() {
        const TOKEN: &str = "BAR";
        const FILEPATH:& str = "FOO\\BAR.txt";
        let mock_writer = MockFileWriter::new();
        let mut cached_writer = CachedWriter::new("FOO", &mock_writer);
        let data = Box::new(String::from_utf8(vec![0u8; 1024]).unwrap());

        for _ in 0..(1024 * 2 - 1) {
            cached_writer.write(TOKEN, data.clone()).unwrap();
        }
        assert!(mock_writer.file_exists(String::from(FILEPATH)) == false);

        for _ in 0..2 {
            cached_writer.write(TOKEN, data.clone()).unwrap();
        }
        assert!(mock_writer.get_file_length(String::from(FILEPATH)) == (1024 *2 + 1) * 1024);
        assert!(mock_writer.get_file_written_times(String::from(FILEPATH)) == 1);

        cached_writer.write(TOKEN, data.clone()).unwrap();
        cached_writer.flush().unwrap();
        assert!(mock_writer.get_file_length(String::from(FILEPATH)) == (1024 *2 + 2) * 1024);
        assert!(mock_writer.get_file_written_times(String::from(FILEPATH)) == 2);
    }
}