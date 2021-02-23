use std::ops::RangeInclusive;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    Capabilities(Vec<Capability>),
    Continue {
        code: Option<ResponseCode>,
        information: Option<String>,
    },
    Done {
        tag: String,
        status: Status,
        code: Option<ResponseCode>,
        information: Option<String>,
    },
    Data {
        status: Status,
        code: Option<ResponseCode>,
        information: Option<String>,
    },
    Expunge(u32),
    Vanished {
        earlier: bool,
        uids: Vec<RangeInclusive<u32>>,
    },
    Fetch(u32, Vec<AttributeValue>),
    MailboxData(MailboxData),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Capability {
    Imap4rev1,
    Auth(String),
    Atom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseCode {
    Alert,
    BadCharset(Option<Vec<String>>),
    Capabilities(Vec<Capability>),
    HighestModSeq(u64), // RFC 4551, section 3.1.1
    Parse,
    PermanentFlags(Vec<String>),
    ReadOnly,
    ReadWrite,
    TryCreate,
    UidNext(u32),
    UidValidity(u32),
    Unseen(u32),
    AppendUid(u32, Vec<UidSetMember>),
    CopyUid(u32, Vec<UidSetMember>, Vec<UidSetMember>),
    UidNotSticky,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UidSetMember {
    UidRange(RangeInclusive<u32>),
    Uid(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeValue {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailboxData {
    Exists(u32),
    Flags(Vec<MailboxFlag>),
    List {
        flags: Vec<String>,
        delimiter: Option<String>,
        name: String,
    },
    Search(Vec<u32>),
    Status {
        mailbox: String,
        status: Vec<StatusAttribute>,
    },
    Recent(u32),
    MetadataSolicited {
        mailbox: String,
        values: Vec<Metadata>,
    },
    MetadataUnsolicited {
        mailbox: String,
        values: Vec<String>,
    },
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum MailboxFlag {
    Answered,
    Flagged,
    Deleted,
    Seen,
    Draft,
    Ext(String),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Metadata {
    pub entry: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StatusAttribute {
    HighestModSeq(u64), // RFC 4551
    Messages(u32),
    Recent(u32),
    UidNext(u32),
    UidValidity(u32),
    Unseen(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    Ok,
    No,
    Bad,
    PreAuth,
    Bye,
}
