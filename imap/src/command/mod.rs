use std::fmt;

/// Commands, without the tag part.
#[derive(Clone, Debug)]
pub enum Command {
    Capability,
    Starttls,
    Login { username: String, password: String },
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::Capability => write!(f, "CAPABILITY"),
            Command::Starttls => write!(f, "STARTTLS"),
            Command::Login { username, password } => write!(f, "LOGIN {} {}", username, password),
        }
    }
}
