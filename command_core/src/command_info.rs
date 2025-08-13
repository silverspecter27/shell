use crate::command_handler::CommandHandler;

pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub min: usize,
    pub max: usize,
    pub handler: &'static dyn CommandHandler,
}

impl CommandInfo {
    pub const fn new(
        name: &'static str,
        description: &'static str,
        aliases: &'static [&'static str],
        min: usize,
        max: usize,
        handler: &'static dyn CommandHandler,
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