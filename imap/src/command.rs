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
    Search {
        criteria: SearchCriteria,
    },
    Fetch {
        // TODO: do sequence-set
        uids: Vec<u32>,
        items: FetchItems,
    },
    UidSearch {
        criteria: SearchCriteria,
    },
    UidFetch {
        // TODO: do sequence-set
        uids: Vec<u32>,
        items: FetchItems,
    },

    #[cfg(feature = "rfc2177-idle")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rfc2177-idle")))]
    Idle,

    #[cfg(feature = "rfc2177-idle")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rfc2177-idle")))]
    Done,
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Command::*;
        match self {
            Login { .. } => write!(f, "LOGIN"),
            _ => <Self as fmt::Display>::fmt(self, f),
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
            Search { criteria } => write!(f, "SEARCH {}", criteria),
            UidSearch { criteria } => write!(f, "UID SEARCH {}", criteria),
            List { reference, mailbox } => write!(f, "LIST {:?} {:?}", reference, mailbox),
            Fetch { uids, items } => write!(
                f,
                "FETCH {} {}",
                uids.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                items
            ),
            UidFetch { uids, items } => write!(
                f,
                "UID FETCH {} {}",
                uids.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                items
            ),

            #[cfg(feature = "rfc2177-idle")]
            Idle => write!(f, "IDLE"),
            #[cfg(feature = "rfc2177-idle")]
            Done => write!(f, "DONE"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SearchCriteria {
    All,
}

impl fmt::Display for SearchCriteria {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use SearchCriteria::*;
        match self {
            All => write!(f, "ALL"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum FetchItems {
    All,
    Fast,
    Full,
}

impl fmt::Display for FetchItems {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use FetchItems::*;
        match self {
            All => write!(f, "ALL"),
            Fast => write!(f, "FAST"),
            Full => write!(f, "FULL"),
        }
    }
}
