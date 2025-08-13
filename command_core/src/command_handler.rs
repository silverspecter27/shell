use crate::command_error::CommandError;

pub trait CommandHandler: Sync + Send {
    fn call(&self, args: &[&str]) -> Result<(), CommandError>;
    fn command_info(&self) -> &'static crate::CommandInfo;
}