use pest::{error::Error, Parser, iterators::{Pair, Pairs}};

use crate::response::*;

#[derive(Parser)]
#[grammar = "parser/rfc3501.pest"]
struct Rfc3501;

pub fn parse_capability(s: &str) -> Result<Capability, Error<Rule>> {
    let mut pairs = Rfc3501::parse(Rule::capability, s)?;
    let pair = pairs.next().unwrap();
    let cap = match pair.as_rule() {
        Rule::capability => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
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
    let mut pairs = Rfc3501::parse(Rule::response, s)?;
    let pair = pairs.next().unwrap();
    Ok(build_response(pair))
}

fn build_response(pair: Pair<Rule>) -> Response {
    match pair.as_rule() {
        Rule::response => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            match pair.as_rule() {
                Rule::response_data => {
                    let mut pairs = pair.into_inner();
                    let pair = pairs.next().unwrap();
                    match pair.as_rule() {
                        Rule::resp_cond_state => {
                            let mut pairs = pair.into_inner();
                            let pair = pairs.next().unwrap();
                            let status = build_status(pair);
                            let mut code = None;
                            let mut information = None;

                            for pair in pairs {
                                if let resp_text = pair.as_rule() {
                                    information = Some(pair.as_str().to_owned());
                                }
                            }
                            Response::Data { status, code, information }
                        }
                        _ => unreachable!("{:?}", pair),
                    }
                }
                _ => unreachable!("{:?}", pair),
            }
        }
        _ => unreachable!("{:?}", pair),
    }
}

fn build_status(pair: Pair<Rule>) -> Status {
    match pair.as_rule() {
        Rule::resp_status => {
            match pair.as_str().to_uppercase().as_str() {
                "OK" => Status::Ok,
                "NO" => Status::No,
                "BAD" => Status::Bad,
                s => unreachable!("invalid status {:?}", s),
            }
        }
        _ => unreachable!("{:?}", pair),
    }
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

    #[test]
    fn test_section_8() {
        // this little exchange is from section 8 of rfc3501
        // https://tools.ietf.org/html/rfc3501#section-8

        assert_eq!(parse_response("* OK IMAP4rev1 Service Ready\r\n"), Ok(Response::Data {
            status: Status::Ok,
            code: None,
            information: Some("IMAP4rev1 Service Ready".to_owned()),
        }));
    }
}
