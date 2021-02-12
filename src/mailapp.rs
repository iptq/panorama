use std::collections::HashMap;

use lettre::SmtpClient;
use anyhow::Result;

use crate::app::AppI;

pub struct MailApp {
    client: SmtpClient,
}

impl AppI for MailApp {
    type Config = ();
}

impl MailApp {
    pub fn new(domain: impl AsRef<str>) -> Result<Self> {
        let client = SmtpClient::new_simple(domain.as_ref())?;
        Ok(MailApp { client })
    }
}
