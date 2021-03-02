//! Module that implements parsers for all of the IMAP types.

mod literal;
mod old;

use std::fmt::Debug;
use std::mem;
use std::str::FromStr;

use pest::{error::Error, iterators::Pair, ParseResult as PestResult, Parser, ParserState};

use crate::response::*;

#[derive(Parser)]
#[grammar = "parser/rfc3501.pest"]

struct Rfc3501;

pub type ParseResult<T, E = Error<Rule>> = Result<T, E>;

pub fn parse_capability(s: impl AsRef<str>) -> ParseResult<Capability> {
    let mut pairs = Rfc3501::parse(Rule::capability, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_capability(pair))
}

pub fn parse_response(s: impl AsRef<str>) -> ParseResult<Response> {
    let mut pairs = Rfc3501::parse(Rule::response, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_response(pair))
}

fn build_response(pair: Pair<Rule>) -> Response {
    assert!(matches!(pair.as_rule(), Rule::response));

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
                Rule::message_data => {
                    let mut pairs = pair.into_inner();
                    let pair = pairs.next().unwrap();
                    let seq: u32 = build_number(pair);

                    let pair = pairs.next().unwrap();
                    match pair.as_rule() {
                        Rule::message_data_expunge => Response::Expunge(seq),
                        Rule::message_data_fetch => {
                            let mut pairs = pair.into_inner();
                            let msg_att = pairs.next().unwrap();
                            let attrs = msg_att.into_inner().map(build_msg_att).collect();
                            Response::Fetch(seq, attrs)
                        }
                        _ => unreachable!("{:#?}", pair),
                    }
                }
                _ => unreachable!("{:#?}", pair),
            }
        }
        Rule::continue_req => {
            let (code, s) = build_resp_text(unwrap1(pair));
            Response::Continue {
                code,
                information: Some(s),
            }
        }
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_resp_text(pair: Pair<Rule>) -> (Option<ResponseCode>, String) {
    assert!(matches!(pair.as_rule(), Rule::resp_text));
    let mut pairs = pair.into_inner();
    let mut pair = pairs.next().unwrap();
    let mut resp_code = None;
    if let Rule::resp_text_code = pair.as_rule() {
        resp_code = build_resp_text_code(pair);
        pair = pairs.next().unwrap();
    }
    assert!(matches!(pair.as_rule(), Rule::text));
    let s = pair.as_str().to_owned();
    (resp_code, s)
}

fn build_msg_att(pair: Pair<Rule>) -> AttributeValue {
    if !matches!(pair.as_rule(), Rule::msg_att_dyn_or_stat) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();

    match pair.as_rule() {
        Rule::msg_att_dynamic => AttributeValue::Flags(pair.into_inner().map(build_flag).collect()),
        Rule::msg_att_static => build_msg_att_static(pair),
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_msg_att_static(pair: Pair<Rule>) -> AttributeValue {
    if !matches!(pair.as_rule(), Rule::msg_att_static) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();

    match pair.as_rule() {
        Rule::msg_att_static_internaldate => {
            AttributeValue::InternalDate(build_string(unwrap1(pair)))
        }
        Rule::msg_att_static_rfc822_size => AttributeValue::Rfc822Size(build_number(unwrap1(pair))),
        Rule::msg_att_static_envelope => AttributeValue::Envelope(build_envelope(unwrap1(pair))),
        // TODO: do this
        Rule::msg_att_static_body => AttributeValue::BodySection {
            section: None,
            index: None,
            data: None,
        },
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_envelope(_pair: Pair<Rule>) -> Envelope {
    // TODO: do this
    Envelope::default()
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
            Rule::resp_text_code => code = build_resp_text_code(pair),
            Rule::text => information = Some(pair.as_str().to_owned()),
            _ => unreachable!("{:#?}", pair),
        }
    }

    (status, code, information)
}

fn build_resp_text_code(pair: Pair<Rule>) -> Option<ResponseCode> {
    if !matches!(pair.as_rule(), Rule::resp_text_code) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next()?;
    Some(match pair.as_rule() {
        Rule::capability_data => ResponseCode::Capabilities(build_capabilities(pair)),
        Rule::resp_text_code_readwrite => ResponseCode::ReadWrite,
        Rule::resp_text_code_uidvalidity => ResponseCode::UidValidity(build_number(unwrap1(pair))),
        Rule::resp_text_code_uidnext => ResponseCode::UidNext(build_number(unwrap1(pair))),
        Rule::resp_text_code_unseen => ResponseCode::Unseen(build_number(unwrap1(pair))),
        // TODO: maybe have an actual type for these flags instead of just string
        Rule::resp_text_code_permanentflags => {
            ResponseCode::PermanentFlags(pair.into_inner().map(|p| p.as_str().to_owned()).collect())
        }
        Rule::resp_text_code_other => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            let a = pair.as_str().to_owned();
            let mut b = None;
            if let Some(pair) = pairs.next() {
                b = Some(pair.as_str().to_owned());
            }
            ResponseCode::Other(a, b)
        }
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

fn build_flag_list(pair: Pair<Rule>) -> Vec<MailboxFlag> {
    if !matches!(pair.as_rule(), Rule::flag_list) {
        unreachable!("{:#?}", pair);
    }

    pair.into_inner().map(build_flag).collect()
}

fn build_flag(mut pair: Pair<Rule>) -> MailboxFlag {
    if matches!(pair.as_rule(), Rule::flag_fetch) {
        let mut pairs = pair.into_inner();
        pair = pairs.next().unwrap();

        if matches!(pair.as_rule(), Rule::flag_fetch_recent) {
            return MailboxFlag::Recent;
        }
    }

    if !matches!(pair.as_rule(), Rule::flag) {
        unreachable!("{:#?}", pair);
    }

    match pair.as_str() {
        "\\Answered" => MailboxFlag::Answered,
        "\\Flagged" => MailboxFlag::Flagged,
        "\\Deleted" => MailboxFlag::Deleted,
        "\\Seen" => MailboxFlag::Seen,
        "\\Draft" => MailboxFlag::Draft,
        // s if s.starts_with("\\") => MailboxFlag::Ext(s.to_owned()),
        // TODO: what??
        s => MailboxFlag::Ext(s.to_owned()),
    }
}

fn build_mailbox_data(pair: Pair<Rule>) -> MailboxData {
    if !matches!(pair.as_rule(), Rule::mailbox_data) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    match pair.as_rule() {
        Rule::mailbox_data_exists => MailboxData::Exists(build_number(unwrap1(pair))),
        Rule::mailbox_data_flags => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            let flags = build_flag_list(pair);
            MailboxData::Flags(flags)
        }
        Rule::mailbox_data_recent => MailboxData::Recent(build_number(unwrap1(pair))),
        Rule::mailbox_data_list => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            let (flags, delimiter, name) = build_mailbox_list(pair);
            MailboxData::List {
                flags,
                delimiter,
                name,
            }
        }
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_mailbox_list(pair: Pair<Rule>) -> (Vec<String>, Option<String>, String) {
    if !matches!(pair.as_rule(), Rule::mailbox_list) {
        unreachable!("{:#?}", pair);
    }

    let mut pairs = pair.into_inner();
    let mut pair = pairs.next().unwrap();

    // let mut flags = Vec::new();
    let flags = if let Rule::mailbox_list_flags = pair.as_rule() {
        let pairs_ = pair.into_inner();
        let mut flags = Vec::new();
        for pair in pairs_ {
            flags.extend(build_mbx_list_flags(pair));
        }
        pair = pairs.next().unwrap();
        flags
    } else {
        Vec::new()
    };

    assert!(matches!(pair.as_rule(), Rule::mailbox_list_string));
    let s = build_nstring(pair);

    pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::mailbox));
    let mailbox = build_string(pair);

    (flags, s, mailbox)
}

fn build_mbx_list_flags(pair: Pair<Rule>) -> Vec<String> {
    assert!(matches!(pair.as_rule(), Rule::mbx_list_flags));
    pair.into_inner()
        .map(|pair| pair.as_str().to_owned())
        .collect()
}

/// Unwraps a singleton pair (a pair that only has one element in its `inner` list)
fn unwrap1(pair: Pair<Rule>) -> Pair<Rule> {
    let mut pairs = pair.into_inner();
    pairs.next().unwrap()
}

/// Extracts a numerical type, generic over anything that could possibly be read as a number
// TODO: should probably restrict this to a few cases
fn build_number<T>(pair: Pair<Rule>) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    if !matches!(pair.as_rule(), Rule::nz_number | Rule::number) {
        unreachable!("not a number {:#?}", pair);
    }
    pair.as_str().parse::<T>().unwrap()
}

/// Wrapper around [build_string][1], except return None for the `nil` case
///
/// [1]: self::build_string
fn build_nstring(pair: Pair<Rule>) -> Option<String> {
    if matches!(pair.as_rule(), Rule::nil) {
        return None;
    }
    Some(build_string(pair))
}

/// Extracts a string-type, discarding the surrounding quotes and unescaping the escaped characters
fn build_string(pair: Pair<Rule>) -> String {
    // TODO: actually get rid of the quotes and escaped chars
    pair.as_str().to_owned()
}

fn parse_literal(s: impl AsRef<str>) -> ParseResult<String> {
    let mut pairs = Rfc3501::parse(Rule::literal, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_literal(pair))
}

fn build_literal(pair: Pair<Rule>) -> String {
    assert!(matches!(pair.as_rule(), Rule::literal));

    let mut pairs = pair.into_inner();
    let _ = pairs.next().unwrap();
    let literal_str = pairs.next().unwrap();
    literal_str.as_str().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::*;
    use pest::Parser;

    #[test]
    fn test_literal() {
        assert_eq!(parse_literal("{7}\r\nhellosu"), Ok("hellosu".to_owned()));
    }

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
                MailboxFlag::Answered,
                MailboxFlag::Flagged,
                MailboxFlag::Deleted,
                MailboxFlag::Seen,
                MailboxFlag::Draft,
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

        assert_eq!(
            parse_response(concat!(
                r#"* 12 FETCH (FLAGS (\Seen) INTERNALDATE "17-Jul-1996 02:44:25 -0700" RFC822.SIZE 4286 ENVELOPE ("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US")("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>") BODY ("TEXT" "PLAIN" ("CHARSET" "US-ASCII") NIL NIL "7BIT" 302892))"#,
                "\r\n",
            )),
            Ok(Response::Fetch(
                12,
                vec![
                    AttributeValue::Flags(vec![MailboxFlag::Seen]),
                    AttributeValue::InternalDate("\"17-Jul-1996 02:44:25 -0700\"".to_owned()),
                    AttributeValue::Rfc822Size(4286),
                    AttributeValue::Envelope(Envelope::default()),
                    AttributeValue::BodySection {
                        section: None,
                        index: None,
                        data: None,
                    },
                ]
            ))
        );
    }
}
