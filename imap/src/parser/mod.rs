//! Module that implements parsers for all of the IMAP types.

mod literal;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::str::FromStr;

use chrono::{DateTime, FixedOffset, TimeZone};
use pest::{error::Error, iterators::Pair, Parser};

use crate::response::*;

use self::literal::literal_internal;

#[derive(Parser)]
#[grammar = "parser/rfc3501.pest"]

struct Rfc3501;

pub type ParseResult<T, E = Error<Rule>> = Result<T, E>;

macro_rules! parse_fail {
    ($($tt:tt)*) => {
        { error!($($tt)*); panic!(); }
    };
}

pub fn parse_capability(s: impl AsRef<str>) -> ParseResult<Capability> {
    let mut pairs = Rfc3501::parse(Rule::capability, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_capability(pair))
}

pub fn parse_streamed_response(s: impl AsRef<str>) -> ParseResult<(Response, usize)> {
    let s = s.as_ref();
    let mut pairs = match Rfc3501::parse(Rule::streamed_response, s) {
        Ok(v) => v,
        Err(e) => {
            // error!("stream failed with len {}: {}", len ,e);
            return Err(e);
        }
    };
    let pair = unwrap1(pairs.next().unwrap());
    let span = pair.as_span();
    let range = span.end() - span.start();
    let response = build_response(pair);
    Ok((response, range))
}

pub fn parse_response(s: impl AsRef<str>) -> ParseResult<Response> {
    let mut pairs = Rfc3501::parse(Rule::response, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_response(pair))
}

fn build_response(pair: Pair<Rule>) -> Response {
    assert!(matches!(pair.as_rule(), Rule::response));
    let pair = unwrap1(pair);
    match pair.as_rule() {
        Rule::response_done => build_response_done(pair),
        Rule::response_data => build_response_data(pair),
        Rule::continue_req => build_continue_req(pair),
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_response_done(pair: Pair<Rule>) -> Response {
    assert!(matches!(pair.as_rule(), Rule::response_done));
    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    match pair.as_rule() {
        Rule::response_tagged => {
            let mut pairs = pair.into_inner();
            let pair = pairs.next().unwrap();
            let tag = pair.as_str().to_owned();

            let pair = pairs.next().unwrap();
            let (status, code, information) = build_resp_cond_state(pair);
            Response::Done(ResponseDone {
                tag,
                status,
                code,
                information,
            })
        }
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_response_data(pair: Pair<Rule>) -> Response {
    assert!(matches!(pair.as_rule(), Rule::response_data));
    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    match pair.as_rule() {
        Rule::resp_cond_state => {
            let (status, code, information) = build_resp_cond_state(pair);
            Response::Data(ResponseData {
                status,
                code,
                information,
            })
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

fn build_continue_req(pair: Pair<Rule>) -> Response {
    assert!(matches!(pair.as_rule(), Rule::continue_req));
    let (code, s) = build_resp_text(unwrap1(pair));
    Response::Continue {
        code,
        information: Some(s),
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
    assert!(matches!(pair.as_rule(), Rule::msg_att_static));

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();

    match pair.as_rule() {
        Rule::msg_att_static_internaldate => {
            AttributeValue::InternalDate(build_date_time(unwrap1(pair)))
        }
        Rule::msg_att_static_rfc822_size => AttributeValue::Rfc822Size(build_number(unwrap1(pair))),
        Rule::msg_att_static_envelope => AttributeValue::Envelope(build_envelope(unwrap1(pair))),
        // TODO: do this
        Rule::msg_att_static_body_structure => AttributeValue::BodySection(BodySection {
            section: None,
            index: None,
            data: None,
        }),
        Rule::msg_att_static_body_section => {
            let mut pairs = pair.into_inner();
            let section = None;
            pairs.next();
            let index = match pairs.peek().unwrap().as_rule() {
                Rule::number => Some(build_number(unwrap1(pairs.next().unwrap()))),
                _ => None,
            };
            let data = build_nstring(pairs.next().unwrap());
            AttributeValue::BodySection(BodySection {
                section,
                index,
                data,
            })
        }
        Rule::msg_att_static_uid => AttributeValue::Uid(build_number(unwrap1(unwrap1(pair)))),
        _ => parse_fail!("{:#?}", pair),
    }
}

fn build_section(pair: Pair<Rule>) -> () {
    assert!(matches!(pair.as_rule(), Rule::section));
}

fn build_envelope(pair: Pair<Rule>) -> Envelope {
    // TODO: do this
    let mut pairs = pair.into_inner();
    let date = build_nstring(unwrap1(pairs.next().unwrap()));
    let subject = build_nstring(unwrap1(pairs.next().unwrap()));

    let address1 = |r: Rule, pair: Pair<Rule>| -> Option<Vec<Address>> {
        assert!(matches!(pair.as_rule(), r));
        let pair = unwrap1(pair);
        match pair.as_rule() {
            Rule::nil => None,
            Rule::env_address1 => Some(pair.into_inner().map(build_address).collect()),
            _ => unreachable!("{:?}", pair),
        }
    };

    let from = address1(Rule::env_from, pairs.next().unwrap());
    let sender = address1(Rule::env_sender, pairs.next().unwrap());
    let reply_to = address1(Rule::env_reply_to, pairs.next().unwrap());
    let to = address1(Rule::env_to, pairs.next().unwrap());
    let cc = address1(Rule::env_cc, pairs.next().unwrap());
    let bcc = address1(Rule::env_bcc, pairs.next().unwrap());
    let in_reply_to = build_nstring(unwrap1(pairs.next().unwrap()));
    let message_id = build_nstring(unwrap1(pairs.next().unwrap()));

    Envelope {
        date,
        subject,
        from,
        sender,
        reply_to,
        to,
        cc,
        bcc,
        in_reply_to,
        message_id,
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
    assert!(matches!(pair.as_rule(), Rule::mailbox_data));

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
        Rule::mailbox_data_search => {
            let uids = pair.into_inner().map(build_number).collect();
            MailboxData::Search(uids)
        }
        _ => unreachable!("{:#?}", pair),
    }
}

fn build_mailbox_list(pair: Pair<Rule>) -> (Vec<String>, Option<String>, String) {
    assert!(matches!(pair.as_rule(), Rule::mailbox_list));

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
    let s = build_nstring(unwrap1(pair));

    pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::mailbox));
    let mailbox = if pair.as_str().to_lowercase() == "inbox" {
        pair.as_str().to_owned()
    } else {
        build_astring(unwrap1(pair))
    };

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
    assert!(matches!(pair.as_rule(), Rule::nz_number | Rule::number));
    pair.as_str().parse::<T>().unwrap()
}

fn build_astring(pair: Pair<Rule>) -> String {
    assert!(matches!(pair.as_rule(), Rule::astring));
    let pair_str = pair.as_str().to_owned();
    let mut pairs = pair.into_inner();
    let rule = pairs.peek().map(|p| p.as_rule());
    if let Some(Rule::string) = rule {
        let pair = pairs.next().unwrap();
        build_string(pair)
    } else {
        pair_str
    }
}

fn build_nstring(pair: Pair<Rule>) -> Option<String> {
    assert!(matches!(pair.as_rule(), Rule::nstring));
    let pair = unwrap1(pair);
    match pair.as_rule() {
        Rule::nil => None,
        Rule::string => Some(build_string(pair)),
        _ => unreachable!(),
    }
}

/// Extracts a string-type, discarding the surrounding quotes and unescaping the escaped characters
fn build_string(pair: Pair<Rule>) -> String {
    assert!(matches!(pair.as_rule(), Rule::string));
    let pair = unwrap1(pair);

    match pair.as_rule() {
        Rule::literal => build_literal(pair),
        // TODO: escaping stuff?
        Rule::quoted => pair
            .as_str()
            .trim_start_matches("\"")
            .trim_end_matches("\"")
            .replace("\\\"", "\"")
            .to_owned(),
        _ => unreachable!(),
    }
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

fn parse_zone(s: impl AsRef<str>) -> ParseResult<FixedOffset> {
    let mut pairs = Rfc3501::parse(Rule::zone, s.as_ref())?;
    let pair = pairs.next().unwrap();
    Ok(build_zone(pair))
}

fn build_zone(pair: Pair<Rule>) -> FixedOffset {
    assert!(matches!(pair.as_rule(), Rule::zone));
    let n = pair.as_str().parse::<i32>().unwrap();
    let sign = if n != 0 { n / n.abs() } else { 1 };
    let h = n.abs() / 100;
    let m = n.abs() % 100;
    FixedOffset::east(sign * (h * 60 + m) * 60)
}

fn build_date_time(pair: Pair<Rule>) -> DateTime<FixedOffset> {
    assert!(matches!(pair.as_rule(), Rule::date_time));

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::date_day_fixed));
    let day = pair.as_str().trim().parse::<u32>().unwrap();

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::date_month));
    let month = match pair.as_str() {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => unreachable!(),
    };

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::date_year));
    let year = pair.as_str().trim().parse::<i32>().unwrap();

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::time));
    let mut parts = pair.as_str().split(':');
    let hour = parts.next().unwrap().parse::<u32>().unwrap();
    let minute = parts.next().unwrap().parse::<u32>().unwrap();
    let second = parts.next().unwrap().parse::<u32>().unwrap();

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::zone));
    let zone = build_zone(pair);

    zone.ymd(year, month, day).and_hms(hour, minute, second)
}

fn build_address(pair: Pair<Rule>) -> Address {
    assert!(matches!(pair.as_rule(), Rule::address));

    let mut pairs = pair.into_inner();
    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::addr_name));
    let name = build_nstring(unwrap1(pair));

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::addr_adl));
    let adl = build_nstring(unwrap1(pair));

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::addr_mailbox));
    let mailbox = build_nstring(unwrap1(pair));

    let pair = pairs.next().unwrap();
    assert!(matches!(pair.as_rule(), Rule::addr_host));
    let host = build_nstring(unwrap1(pair));

    Address {
        name,
        adl,
        mailbox,
        host,
    }
}
