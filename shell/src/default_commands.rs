use chrono::{DateTime, Local};

use command_core::{COMMANDS, CommandError, CommandRegistry};
use command_macro::command;

use colored::*;

use crate::{get_current_user, println_current_user};

#[command(name = "pwd", description = "Print the current directory", max = 0)]
pub fn cmd_pwd(_: &[&str]) -> Result<(), CommandError> {
    match std::env::current_dir() {
        Ok(path) => {
            println!("{}", path.to_str().unwrap_or_default().green());
            Ok(())
        }
        Err(e) => Err(CommandError::CommandFailed(format!("Error retrieving current directory: {}", e)))
    }
}

#[command(name = "whoami", description = "Print the current user", max = 0)]
pub fn cmd_whoami(_: &[&str]) -> Result<(), CommandError> {
    println_current_user!();
    Ok(())
}

#[command(name = "cls", description = "Clears the screen", max = 0)]
pub fn cmd_cls(_: &[&str]) -> Result<(), CommandError> {
    clearscreen::clear()
        .expect("failed to clear screen.");

    Ok(())
}

#[command(name = "time", description = "Shows the current time", max = 0)]
pub fn cmd_time(_: &[&str]) -> Result<(), CommandError> {
    let now: DateTime<Local> = Local::now();
    println!("Time is {}", now.format("%H : %M : %S").to_string());

    Ok(())
}

#[command(name = "exit", description = "Exit the shell", aliases("quit", "bye", max = 0))]
pub fn cmd_exit(_: &[&str]) -> Result<(), CommandError> {
    std::process::exit(0);
}

#[command(name = "help", description = "Displays help information", max = 1)]
pub fn cmd_help(args: &[&str]) -> Result<(), CommandError> {
    match args {
        [] => {
            println!();
            for info_fn in COMMANDS {
                let info = info_fn();
                if info.description.is_empty() {
                    println!("{}", info.name);
                } else {
                    println!("{}:\t{}", info.name, info.description);
                }
            }
            println!();

            Ok(())
        }
        [cmd] => {
            match CommandRegistry::find(cmd) {
                Some(info) => {
                    println!("name: {}", info.name);
                    if !info.description.is_empty() {
                        println!("description: {}", info.description);
                    }
                    if !info.aliases.is_empty() {
                        println!("aliases: {}", info.aliases.join(", "));
                    }
                    Ok(())
                }
                None => Err(CommandError::CommandNotFound(cmd.to_string()))
            }
        }
        _ => unreachable!()
    }
}
