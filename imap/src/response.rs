//! Structs and enums that have to do with responses.

use std::fmt;
use std::ops::RangeInclusive;

use chrono::{DateTime, FixedOffset};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    Capabilities(Vec<Capability>),
    Continue {
        code: Option<ResponseCode>,
        information: Option<String>,
    },
    Done(ResponseDone),
    Data(ResponseData),
    Expunge(u32),
    Vanished {
        earlier: bool,
        uids: Vec<RangeInclusive<u32>>,
    },
    Fetch(u32, Vec<AttributeValue>),
    MailboxData(MailboxData),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseData {
    pub status: Status,
    pub code: Option<ResponseCode>,
    pub information: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseDone {
    pub tag: String,
    pub status: Status,
    pub code: Option<ResponseCode>,
    pub information: Option<String>,
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
    Other(String, Option<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UidSetMember {
    UidRange(RangeInclusive<u32>),
    Uid(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeValue {
    BodySection(BodySection),
    BodyStructure(BodyStructure),
    Envelope(Envelope),
    Flags(Vec<MailboxFlag>),
    InternalDate(DateTime<FixedOffset>),
    ModSeq(u64), // RFC 4551, section 3.3.2
    Rfc822(Option<String>),
    Rfc822Header(Option<String>),
    Rfc822Size(u32),
    Rfc822Text(Option<String>),
    Uid(u32),
}

#[derive(Clone, PartialEq, Eq)]
pub struct BodySection {
    pub section: Option<SectionPath>,
    pub index: Option<u32>,
    pub data: Option<String>,
}

impl fmt::Debug for BodySection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BodySection(section={:?} index={:?} data=<{}>",
            self.section,
            self.index,
            self.data.as_ref().map(|s| s.len()).unwrap_or(0)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyStructure {
    Basic {
        common: BodyContentCommon,
        other: BodyContentSinglePart,
        extension: Option<BodyExtension>,
    },
    Text {
        common: BodyContentCommon,
        other: BodyContentSinglePart,
        lines: u32,
        extension: Option<BodyExtension>,
    },
    Message {
        common: BodyContentCommon,
        other: BodyContentSinglePart,
        envelope: Envelope,
        body: Box<BodyStructure>,
        lines: u32,
        extension: Option<BodyExtension>,
    },
    Multipart {
        common: BodyContentCommon,
        bodies: Vec<BodyStructure>,
        extension: Option<BodyExtension>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyContentSinglePart {
    pub id: Option<String>,
    pub md5: Option<String>,
    pub description: Option<String>,
    pub transfer_encoding: ContentEncoding,
    pub octets: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyContentCommon {
    pub ty: ContentType,
    pub disposition: Option<ContentDisposition>,
    pub language: Option<Vec<String>>,
    pub location: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentType {
    pub ty: String,
    pub subtype: String,
    pub params: BodyParams,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentDisposition {
    pub ty: String,
    pub params: BodyParams,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentEncoding {
    SevenBit,
    EightBit,
    Binary,
    Base64,
    QuotedPrintable,
    Other(String),
}

pub type BodyParams = Option<Vec<(String, String)>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyExtension {
    Num(u32),
    Str(Option<String>),
    List(Vec<BodyExtension>),
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub date: Option<String>,
    pub subject: Option<String>,
    pub from: Option<Vec<Address>>,
    pub sender: Option<Vec<Address>>,
    pub reply_to: Option<Vec<Address>>,
    pub to: Option<Vec<Address>>,
    pub cc: Option<Vec<Address>>,
    pub bcc: Option<Vec<Address>>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    pub name: Option<String>,
    pub adl: Option<String>,
    pub mailbox: Option<String>,
    pub host: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Attribute {
    Body,
    Envelope,
    Flags,
    InternalDate,
    ModSeq, // RFC 4551, section 3.3.2
    Rfc822,
    Rfc822Size,
    Rfc822Text,
    Uid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageSection {
    Header,
    Mime,
    Text,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SectionPath {
    Full(MessageSection),
    Part(Vec<u32>, Option<MessageSection>),
}

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

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MailboxFlag {
    Answered,
    Flagged,
    Deleted,
    Seen,
    Draft,
    Recent,
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
