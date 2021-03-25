//! Module for setting up config files and watchers.
//!
//! One of the primary goals of panorama is to be able to always hot-reload configuration files.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use futures::{future::TryFutureExt, stream::StreamExt};
use inotify::{Inotify, WatchMask};
use tokio::{sync::watch, task::JoinHandle};
use xdg::BaseDirectories;

use crate::report_err;

/// Alias for a MailConfig receiver.
pub type ConfigWatcher = watch::Receiver<Config>;

/// Configuration
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Version of the config to use
    /// (potentially for migration later?)
    pub version: String,

    /// Directory to store mail in
    pub mail_dir: PathBuf,

    /// SQLite database path
    pub db_path: PathBuf,

    /// Mail accounts
    #[serde(rename = "mail")]
    pub mail_accounts: HashMap<String, MailAccountConfig>,
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

    /// TLS
    pub tls: TlsMethod,

    /// Auth
    #[serde(flatten)]
    pub auth: ImapAuth,
}

/// Method of authentication for the IMAP server
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "auth")]
pub enum ImapAuth {
    /// Use plain username/password authentication
    #[serde(rename = "plain")]
    #[allow(missing_docs)]
    Plain { username: String, password: String },
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
    config_home: impl AsRef<Path>,
    config_tx: watch::Sender<Config>,
) -> Result<()> {
    let mut buffer = vec![0; 1024];
    let mut event_stream = inotify.event_stream(&mut buffer)?;
    let config_home = config_home.as_ref().to_path_buf();
    let config_path = config_home.join("panorama.toml");

    // first shot
    {
        let config = read_config(&config_path).await?;
        config_tx.send(config)?;
    }

    debug!("listening for inotify events");
    while let Some(v) = event_stream.next().await {
        let event = v.context("event")?;
        debug!("inotify event: {:?}", event);

        if let Some(name) = event.name {
            let path = PathBuf::from(name);
            let path_c = config_home
                .clone()
                .join(path.clone())
                .canonicalize()
                .context("osu")?;
            if !path_c.exists() {
                debug!("path {:?} doesn't exist", path_c);
                continue;
            }

            // TODO: any better way to do this?
            let config_path_c = config_path.canonicalize().context("cfg_path")?;
            if config_path_c != path_c {
                debug!("did not match {:?} {:?}", config_path_c, path_c);
                continue;
            }

            debug!("reading config from {:?}", path_c);
            let config = read_config(path_c).await.context("read")?;
            // debug!("sending config {:?}", config);
            config_tx.send(config)?;
        }
    }

    Ok(())
}

/// Start the entire config watcher system, and return a [ConfigWatcher][self::ConfigWatcher],
/// which is a cloneable receiver of config update events.
pub fn spawn_config_watcher_system() -> Result<(JoinHandle<()>, ConfigWatcher)> {
    let mut inotify = Inotify::init()?;

    let xdg = BaseDirectories::new()?;
    let config_home = xdg.get_config_home().join("panorama");
    if !config_home.exists() {
        fs::create_dir_all(&config_home)?;
    }
    inotify
        .add_watch(&config_home, WatchMask::CLOSE_WRITE)
        .context("adding watch for config home")?;

    // let config_file_path = config_home.join("panorama.toml");
    // if config_file_path.exists() {
    //     inotify
    //         .add_watch(config_file_path, WatchMask::ALL_EVENTS)
    //         .context("adding watch for config file")?;
    // }
    debug!("watching {:?}", config_home);

    let (config_tx, config_update) = watch::channel(Config::default());
    let handle = tokio::spawn(
        start_inotify_stream(inotify, config_home, config_tx).unwrap_or_else(report_err),
    );
    Ok((handle, config_update))
}
