use std::collections::HashSet;

use chrono::{DateTime, Local};
use panorama_imap::response::*;

/// A record that describes the metadata of an email as it appears in the UI list
#[derive(Clone, Debug, Default)]
pub struct EmailMetadata {
    /// UID if the message has one
    pub uid: Option<u32>,

    /// Whether or not this message is unread
    pub unread: bool,

    /// Date
    pub date: Option<DateTime<Local>>,

    /// Sender
    pub from: String,

    /// Subject
    pub subject: String,
}

impl EmailMetadata {
    /// Construct an EmailMetadata from a list of attributes retrieved from the server
    pub fn from_attrs(attrs: Vec<AttributeValue>) -> Self {
        let mut meta = EmailMetadata::default();

        for attr in attrs {
            match attr {
                AttributeValue::Flags(flags) => {
                    let flags = flags.into_iter().collect::<HashSet<_>>();
                    if !flags.contains(&MailboxFlag::Seen) {
                        meta.unread = true;
                    }
                }
                AttributeValue::Uid(new_uid) => meta.uid = Some(new_uid),
                AttributeValue::InternalDate(new_date) => {
                    meta.date = Some(new_date.with_timezone(&Local));
                }
                AttributeValue::Envelope(Envelope {
                    subject: new_subject,
                    from: new_from,
                    ..
                }) => {
                    if let Some(new_from) = new_from {
                        meta.from = new_from
                            .iter()
                            .filter_map(|addr| addr.name.to_owned())
                            .collect::<Vec<_>>()
                            .join(", ");
                    }
                    if let Some(new_subject) = new_subject {
                        // TODO: probably shouldn't insert quoted-printable here
                        // but this is just the most convenient for it to look right at the moment
                        // there's probably some header that indicates it's quoted-printable
                        // MIME?
                        use quoted_printable::ParseMode;
                        let new_subject =
                            quoted_printable::decode(new_subject.as_bytes(), ParseMode::Robust)
                                .unwrap();
                        let new_subject = String::from_utf8(new_subject).unwrap();
                        meta.subject = new_subject;
                    }
                }
                _ => {}
            }
        }

        meta
    }
}
