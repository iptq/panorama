use std::str::FromStr;

pub enum Response {
    Capabilities(Vec<Capability>),
    Done {
        tag: RequestId,
        status: Status,
        code: Option<ResponseCode>,
        information: Option<String>,
    },
}

impl FromStr for Response {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

pub enum Capability {
    Imap4rev1,
    Auth(String),
    Atom(String),
}

pub struct RequestId(pub String);

pub enum Status {
    Ok,
    No,
}

pub enum ResponseCode {}
