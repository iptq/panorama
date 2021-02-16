//! Module for setting up config files and watchers.
//!
//! One of the primary goals of panorama is to be able to always hot-reload configuration files.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;
use futures::{future::TryFutureExt, stream::StreamExt};
use inotify::{Inotify, WatchMask};
use tokio::{sync::watch, task::JoinHandle};

use crate::report_err;

/// Alias for a MailConfig receiver.
pub type ConfigWatcher = watch::Receiver<Config>;

/// Configuration
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Version of the config to use
    /// (potentially for migration later?)
    pub version: String,

    /// Mail accounts
    #[serde(rename = "mail")]
    pub mail_accounts: Vec<MailAccountConfig>,
}

/// Configuration for a single mail account
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MailAccountConfig {
    /// Imap
    pub imap: ImapConfig,
}

/// Configuring an IMAP server
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImapConfig {
    /// Host of the IMAP server (needs to be hostname for TLS)
    pub server: String,

    /// Port of the IMAP server
    pub port: u16,

    /// Username for authenticating to IMAP
    pub username: String,

    /// Password for authenticating to IMAP
    pub password: String,

    /// TLS
    pub tls: TlsMethod,
}

/// Describes when to perform the TLS handshake
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TlsMethod {
    /// Perform TLS handshake immediately upon connection
    #[serde(rename = "on")]
    On,

    /// Perform TLS handshake after issuing the STARTTLS command
    #[serde(rename = "starttls")]
    Starttls,

    /// Don't perform TLS handshake at all (unsecured)
    #[serde(rename = "off")]
    Off,
}

async fn read_config(path: impl AsRef<Path>) -> Result<Config> {
    let mut file = File::open(path.as_ref())?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let config = toml::from_slice(&contents)?;
    Ok(config)
}

async fn start_inotify_stream(
    mut inotify: Inotify,
    config_tx: watch::Sender<Config>,
) -> Result<()> {
    let mut buffer = vec![0; 1024];
    let mut event_stream = inotify.event_stream(&mut buffer)?;

    while let Some(v) = event_stream.next().await {
        let event = v?;

        debug!("event: {:?}", event);

        if let Some(name) = event.name {
            let path = PathBuf::from(name);
            let config = read_config(path).await?;
            config_tx.send(config)?;
        }
    }

    Ok(())
}

/// Start the entire config watcher system, and return a [ConfigWatcher][self::ConfigWatcher],
/// which is a cloneable receiver of config update events.
pub fn spawn_config_watcher_system() -> Result<(JoinHandle<()>, ConfigWatcher)> {
    let mut inotify = Inotify::init()?;
    inotify.add_watch(".", WatchMask::all())?;

    let (config_tx, config_update) = watch::channel(Config::default());
    let handle = tokio::spawn(start_inotify_stream(inotify, config_tx).unwrap_or_else(report_err));
    Ok((handle, config_update))
}
