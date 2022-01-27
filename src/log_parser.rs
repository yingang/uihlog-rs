extern crate chrono;
use chrono::prelude::*;

use std::sync::mpsc;

const HOUR: i32 = 3600;         // hour in seconds
const MINUTE: i32 = 60;         // minute in seconds

const MAX_LOGLINE_LENGTH: usize = 1024;                   // in bytes, for any line in the original .uihlog file
const TYPICAL_LOGLINE_COUNT: usize = 100_000;

const HEADER_END: &str  = "\x03\x0a";       // ETX (End of Text) + LF (\n)
const HEADER_END_OFFSET: usize = 2;
const LOGGING_END: &str = "\x01\x0a";       // SOH (Start of Heading) + LF (\n)
const LOGGING_END2: &str = "\x01\x0aLOG";   // SOH (Start of Heading) + LF (\n) + "LOG"
const LOGGING_END_OFFSET: usize = 2;
const FIELD_DELIM: char = '\x02';           // STX (Start of Text)

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

pub struct LogParser {
    tz: FixedOffset,

    // cache the last parsed timestamp for performance improvement
    last_timestamp_string: String,
    last_parsed_timestamp: String,
}

impl LogParser {
    pub fn new() -> LogParser {
        LogParser {
            tz: Local.timestamp(0, 0).offset().fix(),
            last_timestamp_string: String::new(),
            last_parsed_timestamp: String::new(),
        }
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
        if let Some(idx) = data.find(HEADER_END) {
            self.parse_header(&data[0..idx]);

            let mut start = idx + HEADER_END_OFFSET;
            loop {
                match data[start..].find(LOGGING_END2) {   // in case there is invalid content in the log description
                    Some(to) => {
                        if let Some(line) = self.parse_line(&data[start .. start + to]) {
                            lines.push(line);
                        }
                        start = start + to + LOGGING_END_OFFSET;
                    }
                    None => {
                        if let Some(to) = data[start..].find(LOGGING_END) {  // for the last line
                            if let Some(line) = self.parse_line(&data[start .. start + to]) {
                                lines.push(line);
                            }
                        }
                        break
                    }
                }
            }
        }
        lines
    }

    fn parse_header(&mut self, header: &str) {
        if let Some(start) = header.find("(UTC") {
            if let Some(end) = header[start..].find(")") {
                self.tz = Self::parse_timezone(&header[start + 4 .. start + end]);
            }
        } else {
            println!("failed to locate timezone info! will use the local timezone instead.")
        }
    }

    fn parse_timezone(tz: &str) -> FixedOffset {
        if let Ok(hh) = &tz[1..3].parse::<i32>() {
            if let Ok(mm) = &tz[4..6].parse::<i32>() {
                if &tz[0..1] == "+" || (*hh == 0 && *mm == 0) {
                    return FixedOffset::east(hh * HOUR + mm * MINUTE);
                } else {
                    return FixedOffset::west(hh * HOUR + mm * MINUTE);
                }
            }
        }
        println!("failed to parse timezone info! will use the local timezone instead.");
        Local.timestamp(0, 0).offset().fix()
    }

    fn parse_line(&mut self, line: &str) -> Option<LogLine> {
        let fields: Vec<&str> = line.split(FIELD_DELIM).collect();
        if fields.len() < LogField::FieldCount.into() {
            println!("invalid log line!");
            return None;
        }

        // much faster than using '+' to contatenate strings (about one order of magnitude difference)
        let mut line = String::with_capacity(MAX_LOGLINE_LENGTH);
        line.push_str(&Self::parse_level(fields[LogField::Level as usize]));
        line.push_str(" ");

        // in case there are unexpected field delimiters ('\x02') in the log description
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
        if let Some(_) = desc.rfind(|c| c == '\n' || c == '\r') {
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

        Some(LogLine {
            src: Self::parse_src(fields[LogField::SrcPidTid as usize]),
            pid: Self::parse_pid(fields[LogField::SrcPidTid as usize]).into(),
            content: Box::new(line),
        })
    }

    // Example: LOG_DEV_WARNING => DEV_WARN, etc.
    fn parse_level(buf: &str) -> String {
        if let Some(ch) = buf.chars().nth(8) {
            return match ch {
                // INFO or WARNING
                'I' | 'W' => {
                    let mut level: String = buf[4..12].into();
                    level.push_str(" ");
                    level
                },
                // ERROR (LOG_TRACE_*** would also fall into this category, let it be...)
                'E' => buf[4..13].into(),
                _ => "UNKNOWN_LEVEL".into(),
            }
        }
        "INVALID_LEVEL".into()
    }

    fn parse_src(buf: &str) -> String {
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

    fn parse_pid<'a>(buf: &'a str) -> &'a str {
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
            self.last_timestamp_string = sec.to_string();
            self.last_parsed_timestamp = utc.with_timezone(&self.tz).format("%y%m%d %H:%M:%S").to_string();
            self.last_parsed_timestamp.clone() + "." + msec
        } else {
            "INVALID_TS".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn timezone_parsing() {
        assert!(LogParser::parse_timezone("+08:00") == FixedOffset::east(8 * HOUR));
        assert!(LogParser::parse_timezone("+00:00") == FixedOffset::east(0 * HOUR));
        assert!(LogParser::parse_timezone("-00:00") == FixedOffset::east(0 * HOUR));
        assert!(LogParser::parse_timezone("-07:30") == FixedOffset::west(7 * HOUR + 30 * MINUTE));
        assert!(LogParser::parse_timezone("BAD_TZ") == Local.timestamp(0, 0).offset().fix());
    }

    #[test]
    fn level_parsing() {
        assert!(LogParser::parse_level("LOG_DEV_WARNING") == "DEV_WARN ");
        assert!(LogParser::parse_level("LOG_SVC_INFO") == "SVC_INFO ");
        assert!(LogParser::parse_level("LOG_SVC_ERROR") == "SVC_ERROR");
        assert!(LogParser::parse_level("LOG_SVC_XFILE") == "UNKNOWN_LEVEL");
        assert!(LogParser::parse_level("LOG_SVC") == "INVALID_LEVEL");
        assert!(LogParser::parse_level("LOG_TRACE_INFO") == "TRACE_INF")
    }

    #[test]
    fn src_parsing() {
        assert!(LogParser::parse_src("BAD_SRCPIDTID") == "INVALID_SRC");
        assert!(LogParser::parse_src("SRC/?\\*:(1:2)") == "SRC_____");
    }

    #[test]
    fn pid_parsing() {
        assert!(LogParser::parse_pid("SRC(1:2)") == "1");
        assert!(LogParser::parse_pid("SRC") == "INVALID_PID");
        assert!(LogParser::parse_pid("SRC(1") == "INVALID_PID");
    }

    #[test]
    fn it_works() {
        let fields1: Vec<&str> = vec!["LOG_DEV_INFO", "1346714491516", "SRC1(1:2)", "file1.cpp", "128", "FOO1", "0X2001", "DESC1",                  "1641013262865"];
        let fields2: Vec<&str> = vec!["LOG_DEV_INFO", "1641013262865", "SRC2(3:4)", "file2.cpp", "256", "FOO2", "0X2002", "DESC2\rMORE\nEVEN MORE", "BAD_TIMESTAMP"];
        let fields3: Vec<&str> = vec!["LOG_DEV_INFO", "1641013262865", "SRC3(5:6)", "file3.cpp", "512", "FOO3", "0X2003", "DESC3"                                  ];

        let mut logfile = String::with_capacity(MAX_LOGLINE_LENGTH);
        logfile.push_str("timezone: (UTC+08:00)");
        logfile.push_str(HEADER_END);
        logfile.push_str(fields1.join(FIELD_DELIM.to_string().as_str()).as_str());
        logfile.push_str(LOGGING_END);
        logfile.push_str(fields2.join(FIELD_DELIM.to_string().as_str()).as_str());
        logfile.push_str(LOGGING_END);
        logfile.push_str(fields3.join(FIELD_DELIM.to_string().as_str()).as_str());
        logfile.push_str(LOGGING_END);

        let mut parser = LogParser::new();
        let (tx, rx) = mpsc::channel::<Vec<LogLine>>();
        parser.parse_async(logfile, tx);
        let lines = rx.recv().unwrap();
        assert!(lines.len() == 2);

        assert!(lines[0].src == "SRC1");
        assert!(lines[0].pid == "1");
        assert!(lines[0].content.as_str() == "DEV_INFO  220101 13:01:02.865 [120904 07:21:31.516] SRC1(1:2) DESC1 [FOO1 file1.cpp 128] [0X2001]\n");

        assert!(lines[1].src == "SRC2");
        assert!(lines[1].pid == "3");
        assert!(lines[1].content.as_str() == "DEV_INFO  INVALID_TS [220101 13:01:02.865] SRC2(3:4) DESC2 MORE EVEN MORE [FOO2 file2.cpp 256] [0X2002]\n");
    }
}
