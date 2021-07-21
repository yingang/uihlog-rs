mod cached_writer;
use cached_writer::CachedWriter;

mod sorted_file_list;
use sorted_file_list::SortedFileList;

mod log_parser;
use log_parser::LogParser;
use log_parser::LogLine;

use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;
use std::thread;

const MAX_LOGFILE_SIZE: usize = 10 * 1024 * 1024;         // 10 MB per .uihlog file
const LOGFILE_BUFFER_SIZE: usize = 2 * MAX_LOGFILE_SIZE;

// 2 or 3 is much faster than other configurations if no file writing (on CPU with 4 physical cores)
const MAX_WORKING_THREADS: usize = 2;

fn read_file(filepath: &Path) -> Option<String> {
    if let Ok(mut f) = File::open(&filepath) {
        let mut data = String::with_capacity(LOGFILE_BUFFER_SIZE);
        if let Ok(_) = f.read_to_string(&mut data) {
            return Some(data)
        }        
    }
    println!("failed to read from file {:?}", &filepath);
    None
}

fn parse_folder(folder: &Path) {
    //let start = SystemTime::now();
    let mut file_list = SortedFileList::new(folder);
    let thread_count = std::cmp::min(file_list.count(), MAX_WORKING_THREADS);

    let mut rxs: VecDeque<Receiver<Vec<LogLine>>> = VecDeque::new();
    for _ in 0..thread_count {
        let (tx, rx) = mpsc::channel::<Vec<LogLine>>();
        let path = file_list.next().unwrap();
        println!("{:?}", &path);
        if let Some(content) = read_file(&path) {
            //println!("start a new thread at {:?} later", SystemTime::now().duration_since(start).unwrap());
            thread::spawn(move || {
                let mut parser = LogParser::new();
                parser.parse_async(content, tx);
            });
            rxs.push_back(rx);
        }
    }

    let mut writer = CachedWriter::new(folder.to_str().unwrap());
    loop {
        if let Some(rx) = rxs.pop_front() {
            let lines = rx.recv().unwrap();
            //println!("a thread finished at {:?} later", SystemTime::now().duration_since(start).unwrap());

            // TODO: could reuse the previously created threads?
            let (tx, rx) = mpsc::channel::<Vec<LogLine>>();
            if let Some(path) = file_list.next() {
                println!("{:?}", &path);
                if let Some(content) = read_file(&path) {
                    //println!("start a new thread at {:?} later", SystemTime::now().duration_since(start).unwrap());
                    thread::spawn(|| {
                        let mut parser = LogParser::new();
                        parser.parse_async(content, tx);
                    });
                    rxs.push_back(rx);
                }
            }
            //let start = SystemTime::now();
            for line in lines {
                writer.write(&line.src, line.content.clone()).unwrap();
                writer.write(&line.pid, line.content.clone()).unwrap();
            }
            //println!("finished writing in {:?}", SystemTime::now().duration_since(start).unwrap());
        } else {
            break
        }
    }
    writer.flush().unwrap();
}

fn parse_file(filepath: &Path) {
    if let Some(content) = read_file(&filepath) {
        let output = PathBuf::from(filepath.to_str().unwrap().to_string() + ".txt");
        if let Ok(mut f) = File::create(&output) {
            let mut parser = LogParser::new();
            let lines = parser.parse_sync(content);
            for line in &lines {
                if let Err(_) = f.write(line.content.as_bytes()) {
                    println!("failed to write to file: {:?}", &output);
                    break
                }
            }
        }
    }
}

fn main() {
    let path = env::args().nth(1).expect("no input file or filepath is specified");
    let path = Path::new(&path);

    let start = SystemTime::now();
    if path.is_dir() {
        parse_folder(path);
    } else if path.is_file() {
        parse_file(path);
    }
    println!("total cost: {:?}", SystemTime::now().duration_since(start).unwrap());
}
