use crate::CommandError;

pub trait ParseArgument<'a>: Sized {
    fn parse(s: &'a str) -> Result<Self, CommandError>;
}

impl<'a> ParseArgument<'a> for &'a str {
    fn parse(s: &'a str) -> Result<Self, CommandError> {
        Ok(s)
    }
}

impl<'a> ParseArgument<'a> for String {
    fn parse(s: &str) -> Result<Self, CommandError> {
        Ok(s.to_string())
    }
}

impl<'a> ParseArgument<'a> for u32 {
    fn parse(s: &str) -> Result<Self, CommandError> {
        s.parse().map_err(|_| CommandError::CommandFailed(format!("Invalid u32: '{}'", s)))
    }
}

impl<'a> ParseArgument<'a> for std::path::PathBuf {
    fn parse(s: &str) -> Result<Self, CommandError> {
        Ok(std::path::PathBuf::from(s))
    }
}
