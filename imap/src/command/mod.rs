use std::fmt;

/// Commands, without the tag part.
#[derive(Clone, Debug)]
pub enum Command {
    Capability,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::Capability => write!(f, "CAPABILITY"),
        }
    }
}
