use std::{env, fs::{self}, io, io::Result as IoResult, os::windows::fs::MetadataExt, path::{Path, PathBuf}};

use command_core::CommandError;
use command_macro::command;
use log::{error, info, warn};

use crate::{get_current_user, println_current_dir};

use colored::*;
use humansize::{format_size, DECIMAL};

#[command(name = "cd", description = "Print the current directory, or change it", max = 1)]
pub fn cmd_cd(args: &[&str]) -> Result<(), CommandError> {
    match args.len() {
        0 => {
            println_current_dir!();
            Ok(())
        }
        1 => {
            let curr_dir = env::current_dir()
                .map_err(|e| CommandError::CommandFailed(format!("Failed to get current directory: {e}")))?;

            let mut new_dir = PathBuf::from(curr_dir);
            new_dir.push(&args[0]);

            match env::set_current_dir(&new_dir) {
                Ok(_) => {
                    println_current_dir!();
                    Ok(())
                }
                Err(e) => Err(CommandError::CommandFailed(format!("Error changing directory: {}", e)))
            }
        }
        _ => unreachable!(),
    }
}

#[command(name = "touch", description = "Makes a new empty file", min = 1, max = 1)]
pub fn cmd_touch(args: &[&str]) -> Result<(), CommandError> {
    use fs::File;

    File::create(args[0])
        .map(|_| ())
        .map_err(|e| CommandError::CommandFailed(format!("Could not create file '{}': {e}", args[0])))
}

#[command(name = "mkdir", description = "Makes a new directory", min = 1, max = 1)]
pub fn cmd_mkdir(args: &[&str]) -> Result<(), CommandError> {
    fs::create_dir(args[0])
        .map_err(|e| CommandError::CommandFailed(format!("Failed to make directory '{}': {e}", args[0])))
}

#[command(name = "rmdir", description = "Removes a given directory (if empty)", min = 1, max = 1)]
pub fn cmd_rmdir(args: &[&str]) -> Result<(), CommandError> {
    fs::remove_dir(args[0])
        .map_err(|e| CommandError::CommandFailed(format!("Failed to remove directory '{}': {e}", args[0])))
}

#[command(name = "rm", description = "Removes a given file or directory (with its contents)", min = 1, max = 1)]
pub fn cmd_rm(args: &[&str]) -> Result<(), CommandError> {
    use std::path::Path;

    let path = Path::new(args[0]);
    if !path.exists() {
        return Err(CommandError::CommandFailed(format!("Path '{}' doesn't exist", path.to_string_lossy())));
    }
    
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path) // fallback for any other possible type
    }
    .map_err(|e| CommandError::CommandFailed(format!("Failed to remove '{}': {e}", path.to_string_lossy())))
}

pub fn redirect_output(mode: &str, target: &str, content: &[u8]) -> IoResult<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut options = OpenOptions::new();
    options.write(true).create(true);

    match mode {
        ">" => { options.truncate(true); }
        ">>" => { options.append(true); }
        _ => panic!("Invalid redirection mode: {}", mode),
    }

    let mut file = options.open(Path::new(target))?;
    file.write_all(content)
}

#[command(name = "cat", description = "Output given files, create if doesn't exist")]
pub fn cmd_cat(args: &[&str]) -> Result<(), CommandError> {
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};
    use std::path::Path;

    let mut files: Vec<(&Path, Vec<u8>)> = Vec::with_capacity(args.len());
    let mut args = args.iter().peekable();
    let mut output_redirected = false;

    while let Some(&arg) = args.next() {
        match arg {
            ">" | ">>" => {
                if output_redirected {
                    return Err(CommandError::CommandFailed("Output already redirected".into()));
                }

                output_redirected = true;

                let Some(&path_str) = args.peek() else {
                    return Err(CommandError::CommandFailed("Missing file name after redirection".into()));
                };
                args.next(); // consume the path

                let mut options = OpenOptions::new();
                
                options
                    .write(true)
                    .create(true);

                match arg {
                    ">" => { options.truncate(true); }
                    ">>" => { options.append(true); }
                    _ => unreachable!(),
                }

                let mut output_file = options.open(path_str)
                    .map_err(|e| CommandError::CommandFailed(format!("Could not open output file `{path_str}`: {e}")))?;

                for (_, contents) in &mut files {
                    output_file.write_all(contents)
                        .map_err(|e| CommandError::CommandFailed(format!("Error writing to output file: {e}")))?;
                }
            }
            path_str => match path_str {
                "-" => {
                    let mut contents = String::new();
                    io::stdin()
                        .read_to_string(&mut contents)
                        .map_err(|e| CommandError::CommandFailed(format!("Failed to read from stdin: {e}")))?;

                    files.push((Path::new("stdin"), contents.into_bytes()));
                }
                _ => {
                    let path = Path::new(path_str);
                    if !path.is_file() {
                        warn!("file '{}' does not exist", path.display());
                        continue;
                    }

                    let mut file = File::open(path)
                        .map_err(|e| CommandError::CommandFailed(format!("Failed to open file `{path_str}`: {e}")))?;

                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)
                        .map_err(|e| CommandError::CommandFailed(format!("Error reading file: {e}")))?;

                    files.push((path, contents));
                }
            }
        }
    }

    if !output_redirected {
        for (path, contents) in &files {
            let name = path.file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| "?".into());

            let text = String::from_utf8_lossy(contents);
            if text.len() > 0 {
                println!();
                info!("[{}]", name);
                print!("\n{}\n", text);
            } else {
                info!("File '{}' is empty.", name);
            }
        }
    }

    Ok(())
}

#[command(name = "ls", description = "Displays files and folders from the current directory", max = 1)]
pub fn cmd_ls(args: &[&str]) -> Result<(), CommandError> {
    let path_buf: PathBuf;
    match args.len() {
        0 => {
            path_buf = env::current_dir()
                .map_err(|e| CommandError::CannotAccessCurrentDirectory(e))?;
        }
        1 => {
            path_buf = PathBuf::from(args[0]);
        }
        _ => unreachable!(),
    };

    let entries = fs::read_dir(&path_buf)
        .map_err(|e| CommandError::DirectoryReadError(path_buf, e))?;

    println!();

    for entry_result in entries {
        match entry_result {
            Ok(entry) => match entry.file_type() {
                Ok(file_type) => {
                    let kind = if file_type.is_file() {
                        "[File]"
                    } else if file_type.is_dir() {
                        "[Dir]"
                    } else if file_type.is_symlink() {
                        "[Symlink]"
                    } else {
                        "[Other]"
                    };
                    println!("{}\t{}", kind, entry.path().display());
                }
                Err(_) => println!("{}", entry.path().display())
            }
            Err(e) => error!("Error reading directory entry: {}", e)
        }
    }

    println!();
    Ok(())
}

#[command(name = "du", description = "Print the size of the file passed", min = 1, max = 1)]
pub fn cmd_du(args: &[&str]) -> Result<(), CommandError> {
    let path = Path::new(args[0]);
    fs::metadata(path)
        .map(|metadata| {
            println!("Sizeof '{}' is: {}", path.display(), format_size(metadata.file_size(), DECIMAL));
        })
        .map_err(|e| CommandError::DirectoryReadError(path.to_path_buf(), e))
}
