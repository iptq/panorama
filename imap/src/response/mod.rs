use std::ops::RangeInclusive;

use crate::types::{
    AttributeValue as AttributeValue_, Capability as Capability_, MailboxDatum as MailboxDatum_,
    RequestId, Response as Response_, ResponseCode as ResponseCode_, State, Status,
};

#[derive(Clone, Debug)]
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
        uids: Vec<RangeInclusive<u32>>,
    },
    Fetch(u32, Vec<AttributeValue>),
    MailboxData(MailboxDatum),
}

impl<'a> From<Response_<'a>> for Response {
    fn from(b: Response_) -> Self {
        use Response_::*;
        match b {
            Capabilities(caps) => {
                Response::Capabilities(caps.into_iter().map(Capability::from).collect())
            }
            Continue { code, information } => Response::Continue {
                code: code.map(ResponseCode::from),
                information: information.map(str::to_owned),
            },
            Done {
                tag,
                status,
                code,
                information,
            } => Response::Done {
                tag,
                status,
                code: code.map(ResponseCode::from),
                information: information.map(str::to_owned),
            },
            Data {
                status,
                code,
                information,
            } => Response::Data {
                status,
                code: code.map(ResponseCode::from),
                information: information.map(str::to_owned),
            },
            Expunge(n) => Response::Expunge(n),
            Vanished {earlier, uids} => Response::Vanished{earlier, uids},
            _ => todo!("nyi: {:?}", b),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Capability {
    Imap4rev1,
    Auth(String),
    Atom(String),
}

impl<'a> From<Capability_<'a>> for Capability {
    fn from(b: Capability_) -> Self {
        use Capability_::*;
        match b {
            Imap4rev1 => Capability::Imap4rev1,
            Auth(s) => Capability::Auth(s.to_owned()),
            Atom(s) => Capability::Atom(s.to_owned()),
        }
    }
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

impl<'a> From<ResponseCode_<'a>> for ResponseCode {
    fn from(b: ResponseCode_) -> Self {
        use ResponseCode_::*;
        match b {
            Alert => ResponseCode::Alert,
            BadCharset(s) => {
                ResponseCode::BadCharset(s.map(|v| v.into_iter().map(str::to_owned).collect()))
            }
            Capabilities(v) => {
                ResponseCode::Capabilities(v.into_iter().map(Capability::from).collect())
            }
            HighestModSeq(n) => ResponseCode::HighestModSeq(n),
            Parse => ResponseCode::Parse,
            _ => todo!("nyi: {:?}", b),
        }
    }
}

#[derive(Clone, Debug)]
pub enum UidSetMember {
    UidRange(RangeInclusive<u32>),
    Uid(u32),
}

#[derive(Clone, Debug)]
pub enum AttributeValue {
}

#[derive(Clone, Debug)]
pub enum MailboxDatum {

}
