use std::fmt;

/// Commands, without the tag part.
#[derive(Clone)]
pub enum Command {
    Capability,
    Starttls,
    Login {
        username: String,
        password: String,
    },
    Select {
        mailbox: String,
    },
    List {
        reference: String,
        mailbox: String,
    },

    #[cfg(feature = "rfc2177-idle")]
    Idle,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Command::*;
        match self {
            Capability => write!(f, "CAPABILITY"),
            Starttls => write!(f, "STARTTLS"),
            Login { .. } => write!(f, "LOGIN"),
            Select { mailbox } => write!(f, "SELECT {}", mailbox),
            List { reference, mailbox } => write!(f, "LIST {:?} {:?}", reference, mailbox),

            #[cfg(feature = "rfc2177-idle")]
            Idle => write!(f, "IDLE"),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Command::*;
        match self {
            Capability => write!(f, "CAPABILITY"),
            Starttls => write!(f, "STARTTLS"),
            Login { username, password } => write!(f, "LOGIN {:?} {:?}", username, password),
            Select { mailbox } => write!(f, "SELECT {}", mailbox),
            List { reference, mailbox } => write!(f, "LIST {:?} {:?}", reference, mailbox),

            #[cfg(feature = "rfc2177-idle")]
            Idle => write!(f, "IDLE"),
        }
    }
}
