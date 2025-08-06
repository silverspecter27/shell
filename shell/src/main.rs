use chrono::Local;
use command_core::{CommandInfo, COMMANDS, CommandRegistry};

use colored::*;

use env_logger::Builder;
use log::{error, Level, LevelFilter};

mod default_commands;
mod file_commands;

pub fn get_current_user() -> String {
    whoami::username()
}

#[macro_export]
macro_rules! print_current_user {
    () => {
        print!("{}", get_current_user().purple());
    };
}
#[macro_export]
macro_rules! println_current_user {
    () => {
        println!("{}", get_current_user().purple());
    };
}
#[macro_export]
macro_rules! print_current_dir {
    () => {
        std::env::current_dir()
            .map(|path| print!("{} is in {}", get_current_user().purple(), path.to_str().unwrap_or_default().green()))
            .unwrap_or_else(|e| error!("retrieving current directory: {}", e));
    };
}
#[macro_export]
macro_rules! println_current_dir {
    () => {
        std::env::current_dir()
            .map(|path| println!("{} is in {}", get_current_user().purple(), path.to_str().unwrap_or_default().green()))
            .unwrap_or_else(|e| error!("retrieving current directory: {}", e));
    };
}

fn main() {
    use std::io::{self, Write};

    Builder::new()
        .filter(None, LevelFilter::Debug)
        .format(|buf, record| {
            let timestamp = Local::now().format("%H:%M:%S");

            let log_line = format!(
                "[{} | {}]: {}",
                timestamp,
                record.level(),
                record.args()
            );

            let colored_line = match record.level() {
                Level::Error => log_line.red().bold(),
                Level::Warn => log_line.yellow().bold(),
                Level::Info => log_line.green(),
                Level::Debug => log_line.blue(),
                Level::Trace => log_line.normal(),
            };

            writeln!(buf, "{}", colored_line)
        })
        .init();

    println_current_dir!();

    loop {
        print!("[sh]$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }

        let mut parts = input.trim().split_whitespace();
        if let Some(cmd) = parts.next() {
            let args: Vec<&str> = parts.collect();

            if let Err(e) = CommandRegistry::execute_command(cmd, &args) {
                error!("{}", e);
            }
        }
    }
}
