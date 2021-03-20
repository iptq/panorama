use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::mail::{EmailMetadata, MailEvent};

/// UI's view of the currently-known mail-related state of all accounts.
#[derive(Clone, Debug, Default)]
pub struct MailStore {
    accounts: Arc<RwLock<HashMap<String, Arc<RwLock<MailAccountState>>>>>,
}

impl MailStore {
    pub fn handle_mail_event(&self, evt: MailEvent) {
        let acct_name = evt.acct_name().to_owned();

        {
            let accounts = self.accounts.read();
            let contains_key = accounts.contains_key(&acct_name);
            std::mem::drop(accounts);

            if !contains_key {
                let mut accounts = self.accounts.write();
                accounts.insert(
                    acct_name.clone(),
                    Arc::new(RwLock::new(MailAccountState::default())),
                );
            }
        }

        let accounts = self.accounts.read();
        if let Some(lock) = accounts.get(&acct_name) {
            let mut state = lock.write();
            state.update(evt);
        }
    }

    pub fn iter_accts(&self) -> Vec<String> {
        self.accounts.read().keys().cloned().collect()
    }

    pub fn folders_of(&self, acct_name: impl AsRef<str>) -> Option<Vec<String>> {
        let accounts = self.accounts.read();
        let lock = accounts.get(acct_name.as_ref())?;
        let state = lock.read();
        Some(state.folders.clone())
    }

    pub fn messages_of(&self, acct_name: impl AsRef<str>) -> Option<Vec<EmailMetadata>> {
        let accounts = self.accounts.read();
        let lock = accounts.get(acct_name.as_ref())?;
        let state = lock.read();
        let mut msgs = Vec::new();
        for uid in state.message_uids.iter() {
            if let Some(meta) = state.message_map.get(uid) {
                msgs.push(meta.clone());
            }
        }
        Some(msgs)
    }
}

#[derive(Debug, Default)]
pub struct MailAccountState {
    pub folders: Vec<String>,
    pub message_uids: Vec<u32>,
    pub message_map: HashMap<u32, EmailMetadata>,
}

impl MailAccountState {
    pub fn update(&mut self, evt: MailEvent) {
        match evt {
            MailEvent::FolderList(_, new_folders) => self.folders = new_folders,
            MailEvent::MessageUids(_, new_uids) => self.message_uids = new_uids,

            MailEvent::UpdateUid(_, uid, attrs) => {
                let meta = EmailMetadata::from_attrs(attrs);
                let uid = meta.uid.unwrap_or(uid);
                self.message_map.insert(uid, meta);
            }
            MailEvent::NewUid(_, uid) => {
                debug!("new msg!");
                self.message_uids.push(uid);
            }
            _ => {}
        }
        // debug!("mail store updated! {:?}", self);
    }
}
