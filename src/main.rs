mod cached_writer;
mod uihlog;
use uihlog::UIHLog;

use std::env;
use std::path::Path;

fn main() {
    let path = env::args().nth(1).expect("no input file or filepath is specified");
    let path = Path::new(&path);

    let mut uihlog = UIHLog::new();
    if path.is_dir() {
        if let Err(_) = uihlog.parse_folder(path) {
            println!("failed to parse folder!");
        }
    } else if path.is_file() {
        if let Err(_) = uihlog.parse_file(path) {
            println!("failed to parse file!");
        }
    }
}
