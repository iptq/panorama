use panorama_imap::response::{AttributeValue, Envelope};

/// Possible events returned from the server that should be sent to the UI
#[derive(Debug)]
#[non_exhaustive]
pub enum MailEvent {
    /// Got the list of folders
    FolderList(String, Vec<String>),

    /// A list of the UIDs in the current mail view
    MessageUids(String, Vec<u32>),

    /// Update the given UID with the given attribute list
    UpdateUid(String, u32, Vec<AttributeValue>),

    /// New message came in with given UID
    NewUid(String, u32),
}

impl MailEvent {
    /// Retrieves the account name that this event is associated with
    pub fn acct_name(&self) -> &str {
        use MailEvent::*;
        match self {
            FolderList(name, _)
            | MessageUids(name, _)
            | UpdateUid(name, _, _)
            | NewUid(name, _) => name,
        }
    }
}
