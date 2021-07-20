extern crate chrono;
use chrono::prelude::*;

use std::sync::mpsc;

const MAX_LOGLINE_LENGTH: usize = 1024;                   // in bytes, for any line in the original .uihlog file
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

pub struct LogLine {
    pub src: String,
    pub pid: String,
    pub content: Box<String>,
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

    pub fn parse_async(&mut self, content: String, sender: mpsc::Sender<Vec<LogLine>>) {
        let lines = self.parse_buffer(&content);
        if let Err(_) = sender.send(lines) {
            println!("failed to send parsed result!");
        }
    }

    pub fn parse_sync(&mut self, content: String) -> Vec<LogLine> {
        self.parse_buffer(&content)
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
}
