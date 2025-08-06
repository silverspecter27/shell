use std::{env, fs::{self}, io::{self, Write}, os::windows::fs::MetadataExt, path::{Path, PathBuf}, sync::Mutex};

use command_core::CommandError;
use command_macro::command;
use log::{error, info, warn};

use crate::{get_current_user, println_current_dir};

use colored::*;
use humansize::{format_size, DECIMAL};

#[command(name = "cd", description = "Print the current directory, or change it", max = 1)]
pub fn cmd_cd(args: &[&str]) -> Result<(), CommandError> {
    match args {
        [] => {
            println_current_dir!();
            Ok(())
        }
        [path] => {
            let curr_dir = env::current_dir()
                .map_err(|e| CommandError::CommandFailed(format!("Failed to get current directory: {e}")))?;

            let mut new_dir = PathBuf::from(curr_dir);
            new_dir.push(path);

            env::set_current_dir(&new_dir)
                .map(|_| println_current_dir!())
                .map_err(|e| CommandError::CommandFailed(format!("Error changing directory: {}", e)))
        }
        _ => unreachable!(),
    }
}

lazy_static::lazy_static! {
    static ref DIR_STACK: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
}

#[command(name = "pushd", description = "Save current directory and change to new one", min = 1, max = 1)]
pub fn cmd_pushd(args: &[&str]) -> Result<(), CommandError> {
    let target = match args {
        [dir] => dir,
        _ => unreachable!(),
    };

    let curr_dir = env::current_dir()
        .map_err(|e| CommandError::CommandFailed(format!("Failed to get current directory: {e}")))?;

    let mut new_dir = PathBuf::from(&curr_dir);
    new_dir.push(target);

    env::set_current_dir(&new_dir)
        .map_err(|e| CommandError::CommandFailed(format!("Error changing directory: {}", e)))?;

    let mut stack = DIR_STACK.lock()
        .map_err(|_| CommandError::CommandFailed("Failed to lock directory stack".to_string()))?;
    stack.push(curr_dir);

    println_current_dir!();
    Ok(())
}

#[command(name = "popd", description = "Pop directory from stack and change to it", max = 0)]
pub fn cmd_popd(_: &[&str]) -> Result<(), CommandError> {
    let mut stack = DIR_STACK.lock().unwrap();
    let dir = stack.pop()
        .ok_or_else(|| CommandError::CommandFailed("Directory stack is empty".to_string()))?;

    env::set_current_dir(&dir)
        .map(|_| println_current_dir!())
        .map_err(|e| CommandError::CommandFailed(format!("Error changing directory: {}", e)))
}

#[command(name = "touch", description = "Makes a new empty file", min = 1)]
pub fn cmd_touch(args: &[&str]) -> Result<(), CommandError> {
    use fs::File;

    for arg in args {
        File::create(arg)
            .map(|_| ())
            .map_err(|e| CommandError::CommandFailed(format!("Could not create file '{}': {e}", arg)))?;
    }

    Ok(())
}

#[command(name = "mkdir", description = "Makes a new directory", min = 1)]
pub fn cmd_mkdir(args: &[&str]) -> Result<(), CommandError> {
    for arg in args {
        fs::create_dir(arg)
            .map_err(|e| CommandError::CommandFailed(format!("Failed to make directory '{}': {e}", arg)))?
    }

    Ok(())
}

#[command(name = "rmdir", description = "Removes a given directory (if empty)", min = 1)]
pub fn cmd_rmdir(args: &[&str]) -> Result<(), CommandError> {
    for arg in args {
        fs::remove_dir(arg)
            .map_err(|e| CommandError::CommandFailed(format!("Failed to remove directory '{}': {e}", arg)))?
    }

    Ok(())
}

#[command(name = "rm", description = "Removes a given file or directory (with its contents)", min = 1)]
pub fn cmd_rm(args: &[&str]) -> Result<(), CommandError> {
    use std::path::Path;

    let mut recursively = false;
    let mut interactive = false;
    let mut verbose = false;
    let mut paths = Vec::new();

    for &arg in args {
        match arg {
            "-r" | "-R" | "--recursive" => {
                recursively = true;
            }
            "-i" | "--interactive" => {
                interactive = true;
            }
            "-d" | "--dir" => {
                recursively = false;
            }
            "-v" | "--verbose" => {
                verbose = true;
            }
            _ => {
                paths.push(arg);
            }
        }
    }

    for path_str in paths {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(CommandError::CommandFailed(format!(
                "Path '{}' doesn't exist",
                path.to_string_lossy()
            )));
        }
        
        if interactive {
            print!("Remove '{}'? [y/N]: ", path.display());
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            let input = input.trim().to_lowercase();
            if input != "y" && input != "yes" {
                if verbose {
                    info!("Skipped '{}'", path.display());
                }
                continue;
            }
        }

        if path.is_dir() {
            if recursively {
                fs::remove_dir_all(path)
            } else {
                return Err(CommandError::CommandFailed(format!(
                    "Cannot remove directory '{}': is a directory (use -r)",
                    path.to_string_lossy()
                )));
            }
        } else {
            fs::remove_file(path)
        }
        .map_err(|e| CommandError::CommandFailed(format!("Failed to remove '{}': {e}", path.to_string_lossy())))?;

        if verbose {
            info!("Removed '{}'", path.display());
        }
    }

    Ok(())
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
    let path_buf = match args {
        [] => {
            env::current_dir()
                .map_err(|e| CommandError::CannotAccessCurrentDirectory(e))?
        }
        [path] => {
            PathBuf::from(path)
        }
        _ => unreachable!(),
    };

    let mut entries: Vec<_> = fs::read_dir(&path_buf)
        .map_err(|e| CommandError::DirectoryReadError(path_buf, e))?
        .collect::<Result<_, _>>()?;

    entries.sort_by_key(|e| e.path());

    if entries.is_empty() {
        info!("The directory is empty");
        return Ok(());
    }

    println!();
    for entry in entries {
        let path = entry.path();
        match entry.file_type() {
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
                println!("{}\t{}", kind, path.display());
            }
            Err(_) => println!("{}", path.display()),
        }
    }
    println!();

    Ok(())
}

#[command(name = "du", description = "Print the size of the file passed", min = 1)]
pub fn cmd_du(args: &[&str]) -> Result<(), CommandError> {
    for arg in args {
        let path = Path::new(arg);
        fs::metadata(path)
            .map(|metadata| {
                println!("Sizeof '{}' is: {}", path.display(), format_size(metadata.file_size(), DECIMAL));
            })
            .map_err(|e| CommandError::DirectoryReadError(path.to_path_buf(), e))?
    }

    Ok(())
}
