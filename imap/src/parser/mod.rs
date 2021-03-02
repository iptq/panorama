mod literal;
mod old;

use anyhow::Result;

use crate::response::*;

use self::literal::literal_internal;

pub fn parse_capability(s: impl AsRef<str>) -> Result<Capability> {
    let s = s.as_ref();
    if s == "IMAP4rev1" {
        Ok(Capability::Imap4rev1)
    } else if s.to_lowercase().starts_with("AUTH=") {
        Ok(Capability::Auth(s[5..].to_owned()))
    } else {
        Ok(Capability::Atom(s.to_owned()))
    }
}

pub fn parse_response(s: impl AsRef<str>) -> Result<Response> {
    let s = s.as_ref();
    let mut parts = s.split(' ');
    let tag = parts.next().unwrap();
    todo!()
}
