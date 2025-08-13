use chrono::{DateTime, Local};

use command_core::{COMMANDS, CommandError, CommandRegistry};
use command_macro::command;

use colored::*;

use crate::{get_current_user, println_current_user};

#[command(name = "pwd", description = "Print the current directory")]
pub fn cmd_pwd() -> Result<(), CommandError> {
    match std::env::current_dir() {
        Ok(path) => {
            println!("{}", path.to_str().unwrap_or_default().green());
            Ok(())
        }
        Err(e) => Err(CommandError::CommandFailed(format!("Error retrieving current directory: {}", e)))
    }
}

#[command(name = "whoami", description = "Print the current user")]
pub fn cmd_whoami() -> Result<(), CommandError> {
    println_current_user!();
    Ok(())
}

#[command(name = "cls", description = "Clears the screen")]
pub fn cmd_cls() -> Result<(), CommandError> {
    clearscreen::clear()
        .expect("failed to clear screen.");

    Ok(())
}

#[command(name = "time", description = "Shows the current time")]
pub fn cmd_time() -> Result<(), CommandError> {
    let now: DateTime<Local> = Local::now();
    println!("Time is {}", now.format("%H : %M : %S").to_string());

    Ok(())
}

#[command(name = "exit", description = "Exit the shell", aliases = ["quit", "bye"])]
pub fn cmd_exit() -> Result<(), CommandError> {
    std::process::exit(0);
}

#[command(name = "help", description = "Displays help information")]
pub fn cmd_help(command: Option<String>) -> Result<(), CommandError> {
    if let Some(command) = command {
        match CommandRegistry::find(command.as_str()) {
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
            None => Err(CommandError::CommandNotFound(command.to_string()))
        }
    } else {
        println!();
        for info in COMMANDS {
            if info.description.is_empty() {
                println!("{}", info.name);
            } else {
                println!("{}:\t{}", info.name, info.description);
            }
        }
        println!();

        Ok(())
    }
}