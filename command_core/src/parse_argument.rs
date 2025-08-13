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

impl<'a> ParseArgument<'a> for char {
    fn parse(s: &'a str) -> Result<Self, CommandError> {
        let mut chars = s.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            Ok(c)
        } else {
            Err(CommandError::CommandFailed(format!("Invalid char: '{}'", s)))
        }
    }
}

impl<'a> ParseArgument<'a> for bool {
    fn parse(s: &str) -> Result<Self, CommandError> {
        match s.to_lowercase().as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(CommandError::CommandFailed(format!("Invalid bool: '{}'", s))),
        }
    }
}

macro_rules! impl_parse_number {
    ($($t:ty),*) => {
        $(
            impl<'a> ParseArgument<'a> for $t {
                fn parse(s: &str) -> Result<Self, CommandError> {
                    s.parse().map_err(|_| CommandError::CommandFailed(format!("Invalid {}: '{}'", stringify!($t), s)))
                }
            }
        )*
    };
}

impl_parse_number!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
impl_parse_number!(f32, f64);

impl<'a> ParseArgument<'a> for std::path::PathBuf {
    fn parse(s: &str) -> Result<Self, CommandError> {
        Ok(std::path::PathBuf::from(s))
    }
}
