use std::{io::Error as IoError, path::PathBuf};

use crate::command_info::CommandInfo;

#[derive(Debug)]
pub enum CommandError {
    TooFewArguments(usize, &'static CommandInfo),
    TooManyArguments(usize, &'static CommandInfo),
    CommandNotFound(String),
    CommandFailed(String),
    InvalidArguments(String),
    CannotAccessCurrentDirectory(IoError),
    DirectoryReadError(PathBuf, IoError),
    FileReadError(PathBuf, IoError),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::TooFewArguments(args_passed, info) => {
                write!(f, "Too few arguments passed '{}' when calling command '{}', the minimum required is '{}'", args_passed, info.name, info.min)
            },
            CommandError::TooManyArguments(args_passed, info) => {
                write!(f, "Too many arguments passed '{}' when calling command '{}', the maximum required is '{}'", args_passed, info.name, info.max)
            },
            CommandError::CommandNotFound(cmd) => {
                write!(f, "Command '{}' not found", cmd)
            },
            CommandError::CommandFailed(e) => {
                write!(f, "{}", e)
            },
            CommandError::InvalidArguments(e) => {
                write!(f, "{}", e)
            }
            CommandError::CannotAccessCurrentDirectory(e) => {
                write!(f, "Could not access the current directory: {}", e)
            },
            CommandError::DirectoryReadError(path, e) => {
                write!(f, "Could not read directory '{}': {}", path.display(), e)
            },
            CommandError::FileReadError(path, e) => {
                write!(f, "Could not read file '{}': {}", path.display(), e)
            },
        }
    }
}

impl From<IoError> for CommandError {
    fn from(err: IoError) -> Self {
        CommandError::CommandFailed(err.to_string())
    }
}