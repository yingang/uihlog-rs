use crate::cached_writer::CachedWriter;

extern crate chrono;
use chrono::prelude::*;

use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_LOGFILE_SIZE: usize = 10 * 1024 * 1024;         // 10 MB per .uihlog file
const MAX_LOGLINE_LENGTH: usize = 1024;                   // in bytes, for any line in the original .uihlog file

const LOGFILE_BUFFER_SIZE: usize = 2 * MAX_LOGFILE_SIZE;
const TYPICAL_LOGLINE_COUNT: usize = 100_000;

enum LogField
{
    Level = 0,
    LocalTS,
    SrcPidTid,
    FileName,
    LineNo,
    Function,
    Uid,
    Description,
    ServerTS,
    FieldCount,
}

impl Into<usize> for LogField {
    fn into(self) -> usize {
        self as usize
    }
}

struct LogLine {
    src: String,
    pid: String,
    content: Box<String>,    // TODO: use shared pointer
}

impl LogLine {
    fn new() -> LogLine {
        LogLine { src: "".into(), pid: "".into(), content: Box::new("".into()) }
    }
}

pub struct UIHLog {
    // cache the last parsed timestamp for performance improvement
    last_timestamp_string: String,
    last_parsed_timestamp: String,
}

impl UIHLog {
    pub fn new() -> UIHLog {
        UIHLog { last_timestamp_string: "".into(), last_parsed_timestamp: "".into() }
    }

    pub fn parse_folder(&mut self, folder: &Path) -> io::Result<()> {
        println!("parse folder: {:?}", folder);

        let mut writer = CachedWriter::new(folder.to_str().unwrap());
        let mut data = String::with_capacity(LOGFILE_BUFFER_SIZE);
        let files = self.get_ordered_file_list(folder)?;
        for file in &files {
            let start = SystemTime::now();
            print!("{:?} ", file);

            let mut f = File::open(file)?;
            data.clear();
            f.read_to_string(&mut data)?;
            let lines = self.parse_buffer(&data);
            for line in lines {
                writer.write(&line.src, line.content.clone())?;
                writer.write(&line.pid, line.content.clone())?;
            }
            println!("{:?}", SystemTime::now().duration_since(start).unwrap());
        }
        let start = SystemTime::now();
        writer.flush()?;
        println!("final flush {:?}", SystemTime::now().duration_since(start).unwrap());

        Ok(())
    }

    pub fn parse_file(&mut self, path: &Path) -> io::Result<()> {
        let start = SystemTime::now();
        print!("parse file: {:?} ", path);
        let f = File::open(path);
        match f {
            Ok(mut f) => {
                let mut data = String::new();
                f.read_to_string(&mut data)?;
                let lines = self.parse_buffer(&data);
                self.save_parsed_file(&lines, path)?;
            },
            Err(_) => {
                println!("failed to open file {:?}", path)
            }
        }
        println!("{:?}", SystemTime::now().duration_since(start).unwrap());
        Ok(())
    }

    fn parse_buffer(&mut self, data: &str) -> Vec<LogLine> {
        let mut lines = Vec::<LogLine>::with_capacity(TYPICAL_LOGLINE_COUNT);
        if let Some(idx) = data.find('\x0a') {
            let mut start = idx + 1;
            loop {
                match data[start..].find("\x01\x0aLOG") {   // in case there is unexpected line delimiters in the log description
                    Some(to) => {
                        lines.push(self.parse_line(&data[start .. start + to]));
                        start = start + to + 2;
                    }
                    None => {
                        if let Some(to) = data[start..].find("\x01\x0a") {  // for the last line
                            lines.push(self.parse_line(&data[start .. start + to]));
                        }
                        break
                    }
                }
            }
        }
        lines
    }

    fn parse_line(&mut self, line: &str) -> LogLine {
        let fields: Vec<&str> = line.split('\x02').collect();
        if fields.len() < LogField::FieldCount.into() {
            println!("invalid log line!");
            return LogLine::new();
        }

        // much faster than using '+' to contatenate strings (about one order of magnitude difference)
        let mut line = String::with_capacity(MAX_LOGLINE_LENGTH);
        line.push_str(&self.parse_level(fields[LogField::Level as usize]));
        line.push_str(" ");

        // in case there are unexpected field delimiters ('\x02') in the log descrition
        let server_ts = if fields.len() > LogField::FieldCount as usize {
            fields.len() - 1
        } else {
            LogField::ServerTS as usize
        };
        line.push_str(&self.parse_timestamp(fields[server_ts]));

        line.push_str(" [");
        line.push_str(&self.parse_timestamp(fields[LogField::LocalTS as usize]));
        line.push_str("] ");

        line.push_str(fields[LogField::SrcPidTid as usize]);
        line.push_str(" ");

        let desc = fields[LogField::Description as usize];
        if let Some(_) = desc.find(|c| c == '\n' || c == '\r') {
            let desc = desc.replace('\n', " ").replace('\r', " ");
            line.push_str(&desc);
        } else {
            line.push_str(&desc);
        }

        line.push_str(" [");
        line.push_str(fields[LogField::Function as usize]);
        line.push_str(" ");
        line.push_str(fields[LogField::FileName as usize]);
        line.push_str(" ");
        line.push_str(fields[LogField::LineNo as usize]);
        line.push_str("] [");
        line.push_str(fields[LogField::Uid as usize]);
        line.push_str("]\n");

        LogLine {
            src: self.parse_src(fields[LogField::SrcPidTid as usize]),
            pid: self.parse_pid(fields[LogField::SrcPidTid as usize]).into(),
            content: Box::new(line),
        }
    }

       // Example: LOG_DEV_WARNING => DEV_WARN, etc.
       fn parse_level(&mut self, buf: &str) -> String {
        if let Some(ch) = buf.chars().nth(8) {
            return match ch {
                // INFO or WARNING
                'I' | 'W' => {
                    let mut level: String = buf[4..12].into();
                    level.push_str(" ");
                    level
                },
                // ERROR
                'E' => buf[4..13].into(),
                _ => "UNKNOWN_LEVEL".into(),
            }
        }
        "INVALID_LEVEL".into()
    }

    fn parse_src(&mut self, buf: &str) -> String {
        if let Some(src_end) = buf.find('(') {
            let src = &buf[..src_end];

            if let Some(_) = src.find(|c| c == '/' || c == '?' || c == '\\' || c == '*' || c == ':') {
                // suppose it's a rare case, no further optimization
                return src.replace('/', "_").replace('?', "_").replace('\\', "_").replace('*', "_").replace(':', "_");
            } else {
                return src.into();
            }
        }
        "INVALID_SRC".into()
    }

    fn parse_pid<'a>(&mut self, buf: &'a str) -> &'a str {
        if let Some(src_end) = buf.find('(') {
            if let Some(pid_end) = buf[src_end..].find(':') {
                let pid = &buf[src_end + 1 .. src_end + pid_end];
                return pid;
            }
        }
        "INVALID_PID"
    }

    fn parse_timestamp(&mut self, buf: &str) -> String {
        let sec = &buf[..buf.len() - 3];
        let msec = &buf[buf.len() - 3..];

        if sec == self.last_timestamp_string {
            return self.last_parsed_timestamp.clone() + "." + msec;
        }

        if let Ok(sec) = sec.parse::<i64>() {
            let naive = NaiveDateTime::from_timestamp(sec, 0);
            let utc: DateTime<Utc> = DateTime::from_utc(naive, Utc);
            let local: DateTime<Local> = DateTime::from(utc);
            self.last_timestamp_string = sec.to_string();
            self.last_parsed_timestamp = local.format("%y%m%d %H:%M:%S").to_string();
            self.last_parsed_timestamp.clone() + "." + msec
        } else {
            "INVALID_TS".into()
        }
    }
    
    fn get_ordered_file_list(&mut self, folder: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut files: Vec<PathBuf> = Vec::new();
        for path in fs::read_dir(folder)? {
            let path = path?.path();
            if path.is_file() && path.extension().unwrap() == "uihlog" {
                files.push(path);
            }
        }

        files.sort_by(|a, b| {
            let idx_a = a.file_stem().unwrap().to_str().unwrap();
            let idx_b = b.file_stem().unwrap().to_str().unwrap();
            idx_a.parse::<i32>().unwrap().cmp(&idx_b.parse::<i32>().unwrap())
            }
        );
        Ok(files)
    }
    
    fn save_parsed_file(&mut self, data: &Vec<LogLine>, path: &Path) -> io::Result<()> {
        let output = PathBuf::from(path.to_str().unwrap().to_string() + ".txt");
        if let Ok(mut f) = File::create(output) {
            for line in data {
                f.write(line.content.as_bytes())?;
            }
        } else {
            println!("failed to open output file!");
        }
        Ok(())
    }
}
