use std::ops::RangeInclusive;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request(pub Vec<u8>, pub Vec<u8>);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AttrMacro {
    All,
    Fast,
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Response {
    Capabilities(Vec<Capability>),
    Continue {
        code: Option<ResponseCode>,
        information: Option<String>,
    },
    Done {
        tag: RequestId,
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
        uids: Vec<std::ops::RangeInclusive<u32>>,
    },
    Fetch(u32, Vec<AttributeValue>),
    MailboxData(MailboxDatum),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    Ok,
    No,
    Bad,
    PreAuth,
    Bye,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UidSetMember {
    UidRange(RangeInclusive<u32>),
    Uid(u32),
}
impl From<RangeInclusive<u32>> for UidSetMember {
    fn from(x: RangeInclusive<u32>) -> Self {
        UidSetMember::UidRange(x)
    }
}
impl From<u32> for UidSetMember {
    fn from(x: u32) -> Self {
        UidSetMember::Uid(x)
    }
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
pub struct Metadata {
    pub entry: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MailboxDatum {
    Exists(u32),
    Flags(Vec<String>),
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

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Capability {
    Imap4rev1,
    Auth(String),
    Atom(String),
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

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AttributeValue {
    BodySection {
        section: Option<SectionPath>,
        index: Option<u32>,
        data: Option<Vec<u8>>,
    },
    BodyStructure(BodyStructure),
    Envelope(Box<Envelope>),
    Flags(Vec<String>),
    InternalDate(String),
    ModSeq(u64), // RFC 4551, section 3.3.2
    Rfc822(Option<Vec<u8>>),
    Rfc822Header(Option<Vec<u8>>),
    Rfc822Size(u32),
    Rfc822Text(Option<Vec<u8>>),
    Uid(u32),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyContentCommon {
    pub ty: ContentType,
    pub disposition: Option<ContentDisposition>,
    pub language: Option<Vec<String>>,
    pub location: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyContentSinglePart {
    pub id: Option<String>,
    pub md5: Option<String>,
    pub description: Option<String>,
    pub transfer_encoding: ContentEncoding,
    pub octets: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentType {
    pub ty: String,
    pub subtype: String,
    pub params: BodyParams,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDisposition {
    pub ty: String,
    pub params: BodyParams,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentEncoding {
    SevenBit,
    EightBit,
    Binary,
    Base64,
    QuotedPrintable,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BodyExtension {
    Num(u32),
    Str(Option<String>),
    List(Vec<BodyExtension>),
}

pub type BodyParams = Option<Vec<(String, String)>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Envelope {
    pub date: Option<Vec<u8>>,
    pub subject: Option<Vec<u8>>,
    pub from: Option<Vec<Address>>,
    pub sender: Option<Vec<Address>>,
    pub reply_to: Option<Vec<Address>>,
    pub to: Option<Vec<Address>>,
    pub cc: Option<Vec<Address>>,
    pub bcc: Option<Vec<Address>>,
    pub in_reply_to: Option<Vec<u8>>,
    pub message_id: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Address {
    pub name: Option<Vec<u8>>,
    pub adl: Option<Vec<u8>>,
    pub mailbox: Option<Vec<u8>>,
    pub host: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}

// Body Structure

pub struct BodyFields {
    pub param: BodyParams,
    pub id: Option<String>,
    pub description: Option<String>,
    pub transfer_encoding: ContentEncoding,
    pub octets: u32,
}

pub struct BodyExt1Part {
    pub md5: Option<String>,
    pub disposition: Option<ContentDisposition>,
    pub language: Option<Vec<String>>,
    pub location: Option<String>,
    pub extension: Option<BodyExtension>,
}

pub struct BodyExtMPart {
    pub param: BodyParams,
    pub disposition: Option<ContentDisposition>,
    pub language: Option<Vec<String>>,
    pub location: Option<String>,
    pub extension: Option<BodyExtension>,
}
