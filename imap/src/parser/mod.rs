use std::fmt::Debug;
use std::str::FromStr;

use pest::{
    error::Error,
    iterators::{Pair, Pairs},
    Parser,
};

use crate::response::*;

#[derive(Parser)]
#[grammar = "parser/rfc3501.pest"]
struct Rfc3501;

pub fn parse_capability(s: impl AsRef<str>) -> Result<Capability, Error<Rule>> {
    let mut pairs = Rfc3501::parse(Rule::capability, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_capability(pair))
}

pub fn parse_response(s: impl AsRef<str>) -> Result<Response, Error<Rule>> {
    let mut pairs = Rfc3501::parse(Rule::response, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_response(pair))
}

fn build_response(pair: Pair<Rule>) -> Response {
    if !matches!(pair.as_rule(), Rule::response) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    match pair.as_rule() {
        Rule::response_done => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            match pair.as_rule() {
                Rule::response_tagged => {
                    let mut pairs = pair.into_inner();
                    let pair = pairs.next().unwrap();
                    let tag = pair.as_str().to_owned();

                    let pair = pairs.next().unwrap();
                    let (status, code, information) = build_resp_cond_state(pair);
                    Response::Done {
                        tag,
                        status,
                        code,
                        information,
                    }
                }
                _ => unreachable!("{:#?}", pair),
            }
        }
        Rule::response_data => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            match pair.as_rule() {
                Rule::resp_cond_state => {
                    let (status, code, information) = build_resp_cond_state(pair);
                    Response::Data {
                        status,
                        code,
                        information,
                    }
                }
                Rule::mailbox_data => Response::MailboxData(build_mailbox_data(pair)),
                Rule::capability_data => Response::Capabilities(build_capabilities(pair)),
                _ => unreachable!("{:#?}", pair),
            }
        }
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_resp_cond_state(pair: Pair<Rule>) -> (Status, Option<ResponseCode>, Option<String>) {
    if !matches!(pair.as_rule(), Rule::resp_cond_state) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    let status = build_status(pair);
    let mut code = None;
    let mut information = None;

    let pair = pairs.next().unwrap();
    let pairs = pair.into_inner();
    for pair in pairs {
        match pair.as_rule() {
            Rule::resp_text_code => code = build_resp_code(pair),
            Rule::text => information = Some(pair.as_str().to_owned()),
            _ => unreachable!("{:#?}", pair),
        }
    }

    (status, code, information)
}

fn build_resp_code(pair: Pair<Rule>) -> Option<ResponseCode> {
    if !matches!(pair.as_rule(), Rule::resp_text_code) {
        unreachable!("{:#?}", pair);
    }

    // panic!("pair: {:#?}", pair);
    debug!("pair: {:#?}", pair);

    let mut pairs = pair.into_inner();
    let pair = pairs.next()?;
    Some(match pair.as_rule() {
        Rule::capability_data => ResponseCode::Capabilities(build_capabilities(pair)),
        Rule::resp_text_code_readwrite => ResponseCode::ReadWrite,
        Rule::resp_text_code_uidvalidity => ResponseCode::UidValidity(build_number(pair)),
        Rule::resp_text_code_unseen => ResponseCode::Unseen(build_number(pair)),
        _ => unreachable!("{:#?}", pair),
    })
}

fn build_capability(pair: Pair<Rule>) -> Capability {
    if !matches!(pair.as_rule(), Rule::capability) {
        unreachable!("{:#?}", pair);
    }

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

fn build_capabilities(pair: Pair<Rule>) -> Vec<Capability> {
    if !matches!(pair.as_rule(), Rule::capability_data) {
        unreachable!("{:#?}", pair);
    }

    pair.into_inner().map(build_capability).collect()
}

fn build_status(pair: Pair<Rule>) -> Status {
    match pair.as_rule() {
        Rule::resp_status => match pair.as_str().to_uppercase().as_str() {
            "OK" => Status::Ok,
            "NO" => Status::No,
            "BAD" => Status::Bad,
            s => unreachable!("invalid status {:?}", s),
        },
        _ => unreachable!("{:?}", pair),
    }
}

fn build_flag_list(pair: Pair<Rule>) -> Vec<Flag> {
    if !matches!(pair.as_rule(), Rule::flag_list) {
        unreachable!("{:#?}", pair);
    }

    pair.into_inner().map(build_flag).collect()
}

fn build_flag(pair: Pair<Rule>) -> Flag {
    if !matches!(pair.as_rule(), Rule::flag) {
        unreachable!("{:#?}", pair);
    }

    match pair.as_str() {
        "\\Answered" => Flag::Answered,
        "\\Flagged" => Flag::Flagged,
        "\\Deleted" => Flag::Deleted,
        "\\Seen" => Flag::Seen,
        "\\Draft" => Flag::Draft,
        s if s.starts_with("\\") => Flag::Ext(s.to_owned()),
        _ => unreachable!("{:#?}", pair.as_str()),
    }
}

fn build_mailbox_data(pair: Pair<Rule>) -> MailboxData {
    if !matches!(pair.as_rule(), Rule::mailbox_data) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    match pair.as_rule() {
        Rule::mailbox_data_exists => MailboxData::Exists(build_number(pair)),
        Rule::mailbox_data_flags => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            let flags = build_flag_list(pair);
            MailboxData::Flags(flags)
        }
        Rule::mailbox_data_recent => MailboxData::Recent(build_number(pair)),
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_number<T>(pair: Pair<Rule>) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    pair.as_str().parse::<T>().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::*;
    use pest::Parser;

    #[test]
    #[rustfmt::skip]
    fn test_capability() {
        assert_eq!(parse_capability("IMAP4rev1"), Ok(Capability::Imap4rev1));
        assert_eq!(parse_capability("LOGINDISABLED"), Ok(Capability::Atom("LOGINDISABLED".to_owned())));
        assert_eq!(parse_capability("AUTH=PLAIN"), Ok(Capability::Auth("PLAIN".to_owned())));
        assert_eq!(parse_capability("auth=plain"), Ok(Capability::Auth("PLAIN".to_owned())));

        assert!(parse_capability("(OSU)").is_err());
        assert!(parse_capability("\x01HELLO").is_err());
    }

    #[test]
    #[rustfmt::skip]
    fn test_nil() {
        assert!(Rfc3501::parse(Rule::nil, "NIL").is_ok());
        assert!(Rfc3501::parse(Rule::nil, "anything else").is_err());
    }

    #[test]
    fn test_section_8() {
        // this little exchange is from section 8 of rfc3501
        // https://tools.ietf.org/html/rfc3501#section-8

        assert_eq!(
            parse_response("* OK IMAP4rev1 Service Ready\r\n"),
            Ok(Response::Data {
                status: Status::Ok,
                code: None,
                information: Some("IMAP4rev1 Service Ready".to_owned()),
            })
        );

        assert_eq!(
            parse_response("a001 OK LOGIN completed\r\n"),
            Ok(Response::Done {
                tag: "a001".to_owned(),
                status: Status::Ok,
                code: None,
                information: Some("LOGIN completed".to_owned()),
            })
        );

        assert_eq!(
            parse_response("* 18 EXISTS\r\n"),
            Ok(Response::MailboxData(MailboxData::Exists(18)))
        );

        assert_eq!(
            parse_response("* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n"),
            Ok(Response::MailboxData(MailboxData::Flags(vec![
                Flag::Answered,
                Flag::Flagged,
                Flag::Deleted,
                Flag::Seen,
                Flag::Draft,
            ])))
        );

        assert_eq!(
            parse_response("* 2 RECENT\r\n"),
            Ok(Response::MailboxData(MailboxData::Recent(2)))
        );

        assert_eq!(
            parse_response("* OK [UNSEEN 17] Message 17 is the first unseen message\r\n"),
            Ok(Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::Unseen(17)),
                information: Some("Message 17 is the first unseen message".to_owned()),
            })
        );

        assert_eq!(
            parse_response("* OK [UIDVALIDITY 3857529045] UIDs valid\r\n"),
            Ok(Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::UidValidity(3857529045)),
                information: Some("UIDs valid".to_owned()),
            })
        );

        assert_eq!(
            parse_response("a002 OK [READ-WRITE] SELECT completed\r\n"),
            Ok(Response::Done {
                tag: "a002".to_owned(),
                status: Status::Ok,
                code: Some(ResponseCode::ReadWrite),
                information: Some("SELECT completed".to_owned()),
            })
        );

        // assert_eq!(
        //     parse_response(concat!(
        //         r#"* 12 FETCH (FLAGS (\Seen) INTERNALDATE "17-Jul-1996 02:44:25 -0700" RFC822.SIZE 4286 ENVELOPE ("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US") ("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>") BODY ("TEXT" "PLAIN" ("CHARSET" "US-ASCII") NIL NIL "7BIT" 3028 92))"#,
        //         "\r\n",
        //     )),
        //     Ok(Response::Fetch(12, vec![]))
        // );
    }
}
