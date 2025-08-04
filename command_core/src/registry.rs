use linkme::distributed_slice;
use crate::{command_info::CommandInfo, CommandError};

#[distributed_slice]
pub static COMMANDS: [fn() -> &'static CommandInfo] = [..];

pub struct CommandRegistry;

impl CommandRegistry {
    pub fn find(name: &str) -> Option<&'static CommandInfo> {
        COMMANDS.iter()
            .find_map(|&info_fn| {
                let info = info_fn();
                if info.name == name || info.aliases.iter().any(|a| a == &name) {
                    Some(info)
                } else {
                    None
                }
            })
    }

    pub fn execute_command(name: &str, args: &[&str]) -> Result<(), CommandError> {
        match CommandRegistry::find(name) {
            Some(info) => {
                if args.len() < info.min {
                    return Err(CommandError::TooFewArguments(args.len(), info));
                }
                if args.len() > info.max && info.max != usize::MAX {
                    return Err(CommandError::TooManyArguments(args.len(), info));
                }

                (info.handler)(&args)
            }
            None => Err(CommandError::CommandNotFound(name.to_string()))
        }
    }

    pub fn all() -> impl Iterator<Item = &'static CommandInfo> {
        COMMANDS.iter().map(|&f| f())
    }
}