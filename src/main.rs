mod cached_writer;
use cached_writer::CachedWriter;

mod sorted_file_list;
use sorted_file_list::SortedFileList;

mod log_parser;
use log_parser::LogParser;
use log_parser::LogLine;

use std::collections::VecDeque;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;
use std::thread;

// 2 or 3 is much faster than other configurations if no file writing (on CPU with 4 physical cores)
const MAX_WORKING_THREADS: usize = 2;

fn read_file(filepath: &Path) -> Option<String> {
    if let Ok(data) = fs::read(filepath) {
        let data = String::from_utf8_lossy(&data);  // consider log file with invalid UTF8 content
        return Some(data.into_owned())
    }
    println!("failed to read from file {:?}", &filepath);
    None
}

fn create_worker_thread(file_list: &mut SortedFileList) -> Option<Receiver<Vec<LogLine>>> {
    let (tx, rx) = mpsc::channel::<Vec<LogLine>>();
    if let Some(path) = file_list.next() {
        println!("{:?}", &path.as_path().file_name().unwrap());
        if let Some(content) = read_file(&path) {
            thread::spawn(move || {
                let mut parser = LogParser::new();
                parser.parse_async(content, tx);
            });
            return Some(rx);
        }
    }
    None
}

fn parse_folder(folder: &Path, pid_output: bool) -> io::Result<()> {
    if need_pid_output() {
        println!("pid output is enabled");
    }

    //let start = SystemTime::now();
    let mut file_list = SortedFileList::new(folder);
    let mut rxs: VecDeque<Receiver<Vec<LogLine>>> = VecDeque::new();

    let thread_count = std::cmp::min(file_list.count(), MAX_WORKING_THREADS);
    for _ in 0..thread_count {
        if let Some(rx) = create_worker_thread(&mut file_list) {
            //println!("start a new thread at {:?} later", SystemTime::now().duration_since(start).unwrap());
            rxs.push_back(rx);
        } else {
            break
        }
    }

    let mut writer = CachedWriter::new(folder.to_str().unwrap());
    loop {
        if let Some(rx) = rxs.pop_front() {
            let lines = rx.recv().unwrap();
            //println!("a thread finished at {:?} later", SystemTime::now().duration_since(start).unwrap());

            if let Some(rx) = create_worker_thread(&mut file_list) {
                //println!("start a new thread at {:?} later", SystemTime::now().duration_since(start).unwrap());
                rxs.push_back(rx);
            }

            //let start = SystemTime::now();
            for line in lines {
                if pid_output {
                    writer.write(&line.pid, line.content.clone())?;
                }
                writer.write(&line.src, line.content)?;
            }
            //println!("finished writing in {:?}", SystemTime::now().duration_since(start).unwrap());
        } else {
            break
        }
    }
    writer.flush()?;
    Ok(())
}

fn parse_file(filepath: &Path) -> io::Result<()> {
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

fn need_pid_output() -> bool {
    match env::args().nth(2) {
        Some(pid_output) => match pid_output.as_str() {
            "1" => true,
            _ => false,
        },
        None => false,
    }
}

fn main() {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("uihlog reloaded in Rust v{} YG", VERSION);

    if let Some(path) = env::args().nth(1) {
        let path = Path::new(&path);

        let start = SystemTime::now();
        if path.is_dir() {
            //let pid_output = env::var("UIHLOG_ENABLE_PID_OUTPUT").is_ok();
            if let Err(_) = parse_folder(path, need_pid_output()) {
                println!("failed to parse the folder");
            }
        } else if path.is_file() {
            if let Err(_) = parse_file(path) {
                println!("failed to parse the file");
            }
        }
        println!("total cost: {:?}", SystemTime::now().duration_since(start).unwrap());
    } else {
        println!("no input file or filepath is specified");
    }
}
