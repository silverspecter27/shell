use crate::command_error::CommandError;

pub type CommandFn = fn(&[&str]) -> Result<(), CommandError>;

#[derive(Debug, PartialEq, Eq)]
pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub max: usize,
    pub min: usize,
    pub handler: CommandFn,
}

impl CommandInfo {
    pub const fn new(
        name: &'static str,
        description: &'static str,
        aliases: &'static [&'static str],
        min: usize,
        max: usize,
        handler: CommandFn,
    ) -> Self {
        Self {
            name,
            description,
            aliases,
            min,
            max,
            handler,
        }
    }
}