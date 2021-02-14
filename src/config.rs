//! Module for setting up config files and watchers.
//!
//! One of the primary goals of panorama is to be able to always hot-reload configuration files.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::{sync::watch, task::JoinHandle};
use xdg::BaseDirectories;

/// Alias for a MailConfig receiver.
pub type ConfigWatcher = watch::Receiver<Option<Config>>;

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
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct MailAccountConfig {
    /// Imap
    pub imap: ImapConfig,
}

/// Configuring an IMAP server
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct ImapConfig {
    /// Host of the IMAP server (needs to be hostname for TLS)
    pub server: String,

    /// Port of the IMAP server
    pub port: u16,

    /// Username for authenticating to IMAP
    pub username: String,

    /// Password for authenticating to IMAP
    pub password: String,
}

/// Spawns a notify::RecommendedWatcher to watch the XDG config directory. Whenever the config file
/// is updated, the config file is parsed and sent to the receiver.
fn start_watcher() -> Result<(RecommendedWatcher, Receiver<DebouncedEvent>)> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Duration::from_secs(5))?;

    let xdg = BaseDirectories::new()?;
    let config_home = xdg.get_config_home();
    debug!("config_home: {:?}", config_home);
    watcher
        .watch(config_home.join("panorama"), RecursiveMode::Recursive)
        .context("could not watch config_home")?;

    Ok((watcher, rx))
}

async fn read_config(path: impl AsRef<Path>) -> Result<Config> {
    let mut file = File::open(path.as_ref())?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let config = toml::from_slice(&contents)?;
    Ok(config)
}

/// The inner loop of the watcher, which is responsible for taking events received by the watcher
/// and trying to parse and return the config.
///
/// This exists so all errors are able to be caught in one go.
async fn watcher_loop(
    fs_events: Receiver<DebouncedEvent>,
    config_tx: watch::Sender<Option<Config>>,
) -> Result<()> {
    // first try opening the config file directly when the program is opened
    // (so the config isn't blank until the user touches the config file)
    let xdg = BaseDirectories::new()?;
    if let Some(config_path) = xdg.find_config_file("panorama/panorama.toml") {
        debug!("found config at {:?}", config_path);
        let config = read_config(config_path).await?;
        debug!("read config: {:?}, sending to output", config);
        config_tx.send(Some(config))?;
    }

    // start listening for events from the notify::Watcher
    for event in fs_events {
        debug!("new event: {:?}", event);
        use notify::DebouncedEvent::*;
        match event {
            NoticeWrite(path) | Write(path) => {
                let config = read_config(path).await?;
                config_tx.send(Some(config))?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Start the entire config watcher system, and return a [ConfigWatcher][self::ConfigWatcher],
/// which is a cloneable receiver of config update events.
pub fn spawn_config_watcher() -> Result<(JoinHandle<()>, ConfigWatcher)> {
    let (watcher, config_rx) = start_watcher()?;
    let (config_tx, config_update) = watch::channel(None);

    let config_thread = tokio::spawn(async move {
        let _watcher = watcher;
        match watcher_loop(config_rx, config_tx).await {
            Ok(_) => {}
            Err(err) => {
                debug!("config watcher bugged: {:?}", err);
            }
        }
    });

    Ok((config_thread, config_update))
}
