use std::ops::RangeInclusive;

#[derive(Clone, Debug)]
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
    MailboxData(MailboxDatum),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Capability {
    Imap4rev1,
    Auth(String),
    Atom(String),
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub enum UidSetMember {
    UidRange(RangeInclusive<u32>),
    Uid(u32),
}

#[derive(Clone, Debug)]
pub enum AttributeValue {}

#[derive(Clone, Debug)]
pub enum MailboxDatum {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    Ok,
    No,
    Bad,
    PreAuth,
    Bye,
}
