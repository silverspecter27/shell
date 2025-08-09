use chrono::Local;
use command_core::{CommandError, CommandInfo, CommandRegistry, COMMANDS};

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
        print!("{}", get_current_user().purple())
    };
}
#[macro_export]
macro_rules! println_current_user {
    () => {
        println!("{}", get_current_user().purple())
    };
}
#[macro_export]
macro_rules! print_current_dir {
    () => {
        std::env::current_dir()
            .map(|path| print!("{} is in {}", get_current_user().purple(), path.to_str().unwrap_or_default().green()))
            .unwrap_or_else(|e| error!("retrieving current directory: {}", e))
    };
}
#[macro_export]
macro_rules! println_current_dir {
    () => {
        std::env::current_dir()
            .map(|path| println!("{} is in {}", get_current_user().purple(), path.to_str().unwrap_or_default().green()))
            .unwrap_or_else(|e| error!("retrieving current directory: {}", e))
    };
}

pub fn call_executable(name: &str, args: &[&str]) -> Result<(), CommandError> {
    use std::io::ErrorKind;

    std::process::Command::new(name)
        .args(args)
        .spawn()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => CommandError::CommandNotFound(format!("{}", name)),
            ErrorKind::PermissionDenied => CommandError::CommandFailed(format!("Permission denied for '{}'", name)),
            _ => CommandError::CommandFailed(format!("{}", e)),
        })?
        .wait()
        .map_err(CommandError::from)
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(CommandError::CommandFailed(format!(
                    "Program '{}' exited with code: '{}'",
                    name,
                    status.code().unwrap_or(-1)
                )))
            }
        })
}

fn main() {
    use std::io::{self, Write};

    _ = enable_ansi_support::enable_ansi_support();

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

            CommandRegistry::execute_command(cmd, &args)
                .or_else(|e| match e {
                    CommandError::CommandNotFound(_) => call_executable(cmd, &args),
                    other => Err(other),
                })
                .map_err(|e| error!("{}", e))
                .ok();
        }
    }
}
