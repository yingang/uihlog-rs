use crate::buffered_output::BufferedOutput;
use crate::file_system::{read_file, RealFileWriter};
use crate::log_parser::{LogLine, LogParser};
use crate::sorted_file_list::SortedFileList;

use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

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

pub fn parse_folder(folder: &Path, pid_output: bool) -> io::Result<()> {
    if pid_output {
        println!("pid output is enabled");
    }

    let mut file_list = SortedFileList::new(folder);
    let mut rxs: VecDeque<Receiver<Vec<LogLine>>> = VecDeque::new();

    let thread_count = std::cmp::min(file_list.count(), MAX_WORKING_THREADS);
    for _ in 0..thread_count {
        if let Some(rx) = create_worker_thread(&mut file_list) {
            rxs.push_back(rx);
        } else {
            println!("failed to initialize parsing tasks");
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