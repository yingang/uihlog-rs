mod buffered_output;
mod file_system;
mod log_parser;
mod sorted_file_list;

use buffered_output::BufferedOutput;
use file_system::{read_file, RealFileWriter};
use log_parser::{LogLine, LogParser};
use sorted_file_list::SortedFileList;

use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::SystemTime;

// 2 or 3 is much faster than other configurations if no file writing (on CPU with 4 physical cores)
const MAX_WORKING_THREADS: usize = 2;

fn create_worker_thread(file_list: &mut SortedFileList) -> Option<Receiver<Vec<LogLine>>> {
    if let Some(path) = file_list.next() {
        println!("{:?}", &path.as_path().file_name().unwrap());
        if let Some(content) = read_file(&path) {
            let (tx, rx) = mpsc::channel::<Vec<LogLine>>();
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

    let mut file_list = SortedFileList::new(folder);
    let mut rxs: VecDeque<Receiver<Vec<LogLine>>> = VecDeque::new();

    let thread_count = std::cmp::min(file_list.count(), MAX_WORKING_THREADS);
    for _ in 0..thread_count {
        if let Some(rx) = create_worker_thread(&mut file_list) {
            rxs.push_back(rx);
        } else {
            break;
        }
    }

    let writer = RealFileWriter::new();
    let mut output = BufferedOutput::new(folder.to_str().unwrap(), &writer);
    loop {
        if let Some(rx) = rxs.pop_front() {
            let lines = rx.recv().unwrap();

            if let Some(rx) = create_worker_thread(&mut file_list) {
                rxs.push_back(rx);
            }

            for line in lines {
                if pid_output {
                    output.send(&line.pid, line.content.clone())?;
                }
                output.send(&line.src, line.content)?;
            }
        } else {
            break;
        }
    }
    output.flush()?;
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
    println!("uihlog reloaded in Rust v{}", VERSION);

    let path = match env::args().nth(1) {
        Some(path) => Path::new(&path).to_owned(),
        None => env::current_dir().unwrap(),
    };

    let start = SystemTime::now();
    if path.is_dir() {
        if let Err(_) = parse_folder(&path, need_pid_output()) {
            println!("failed to parse the folder");
        }
    } else if path.is_file() {
        if let Err(_) = parse_file(&path) {
            println!("failed to parse the file");
        }
    }
    println!(
        "total cost: {:?}",
        SystemTime::now().duration_since(start).unwrap()
    );
}
