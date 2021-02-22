use pest::{error::Error, Parser};

use crate::response::*;

#[derive(Parser)]
#[grammar = "parser/rfc3501.pest"]
struct Rfc3501;

pub fn parse_capability(s: &str) -> Result<Capability, Error<Rule>> {
    let mut pairs = Rfc3501::parse(Rule::capability, s)?;
    let pair = pairs.next().unwrap();
    let cap = match pair.as_rule() {
        Rule::capability => {
            let mut inner = pair.into_inner();
            let pair = inner.next().unwrap();
            match pair.as_rule() {
                Rule::auth_type => Capability::Auth(pair.as_str().to_uppercase().to_owned()),
                Rule::atom => match pair.as_str() {
                    "IMAP4rev1" => Capability::Imap4rev1,
                    s => Capability::Atom(s.to_uppercase().to_owned()),
                },
                _ => unreachable!("{:?}", pair),
            }
        }
        _ => unreachable!("{:?}", pair),
    };
    Ok(cap)
}

pub fn parse_response(s: &str) -> Result<Response, Error<Rule>> {
    todo!()
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use crate::response::*;
    use pest::Parser;

    #[test]
    fn test_capability() {
        assert_eq!(parse_capability("IMAP4rev1"), Ok(Capability::Imap4rev1));
        assert_eq!(parse_capability("LOGINDISABLED"), Ok(Capability::Atom("LOGINDISABLED".to_owned())));
        assert_eq!(parse_capability("AUTH=PLAIN"), Ok(Capability::Auth("PLAIN".to_owned())));
        assert_eq!(parse_capability("auth=plain"), Ok(Capability::Auth("PLAIN".to_owned())));

        assert!(parse_capability("(OSU)").is_err());
        assert!(parse_capability("\x01HELLO").is_err());
    }

    #[test]
    fn test_nil() {
        assert!(Rfc3501::parse(Rule::nil, "NIL").is_ok());
        assert!(Rfc3501::parse(Rule::nil, "anything else").is_err());
    }
}
