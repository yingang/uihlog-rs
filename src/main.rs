mod buffered_output;
mod file_parser;
mod file_system;
mod folder_parser;
mod log_parser;
mod sorted_file_list;

use folder_parser::parse_folder;
use file_parser::parse_file;

use std::env;
use std::path::Path;
use std::time::SystemTime;

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
