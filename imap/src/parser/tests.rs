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
        Ok(Response::Data(ResponseData {
            status: Status::Ok,
            code: None,
            information: Some("IMAP4rev1 Service Ready".to_owned()),
        }))
    );

    assert_eq!(
        parse_response("a001 OK LOGIN completed\r\n"),
        Ok(Response::Done(ResponseDone {
            tag: "a001".to_owned(),
            status: Status::Ok,
            code: None,
            information: Some("LOGIN completed".to_owned()),
        }))
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
        Ok(Response::Data(ResponseData {
            status: Status::Ok,
            code: Some(ResponseCode::Unseen(17)),
            information: Some("Message 17 is the first unseen message".to_owned()),
        }))
    );

    assert_eq!(
        parse_response("* OK [UIDVALIDITY 3857529045] UIDs valid\r\n"),
        Ok(Response::Data(ResponseData {
            status: Status::Ok,
            code: Some(ResponseCode::UidValidity(3857529045)),
            information: Some("UIDs valid".to_owned()),
        }))
    );

    assert_eq!(
        parse_response("a002 OK [READ-WRITE] SELECT completed\r\n"),
        Ok(Response::Done(ResponseDone {
            tag: "a002".to_owned(),
            status: Status::Ok,
            code: Some(ResponseCode::ReadWrite),
            information: Some("SELECT completed".to_owned()),
        }))
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
                AttributeValue::InternalDate("17-Jul-1996 02:44:25 -0700".to_owned()),
                AttributeValue::Rfc822Size(4286),
                AttributeValue::Envelope(Envelope {
                    date: Some("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)".to_owned()),
                    subject: Some("IMAP4rev1 WG mtg summary and minutes".to_owned()),
                    from: None,
                    sender: None,
                    reply_to: None,
                    to: None,
                    cc: None,
                    bcc: None,
                    in_reply_to: None,
                    message_id: Some("<B27397-0100000@cac.washington.edu>".to_owned()),
                }),
                AttributeValue::BodySection {
                    section: None,
                    index: None,
                    data: None,
                },
            ]
        ))
    );
}
