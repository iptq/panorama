use std::fs::File;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::io::Read;
use std::path::Path;

use anyhow::{Result, Context};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::watch;
use xdg::BaseDirectories;

pub type ConfigWatcher = watch::Receiver<Option<MailConfig>>;

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct MailConfig {
    pub server: String,
    pub port: u16,

    pub username: String,
    pub password: String,
}

/// Spawns a notify::RecommendedWatcher to watch the XDG config directory. Whenever the config file
/// is updated, the config file is parsed and sent to the receiver.
fn start_watcher() -> Result<(
    RecommendedWatcher,
    Receiver<DebouncedEvent>,
)> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Duration::from_secs(5))?;

    let xdg = BaseDirectories::new()?;
    let config_home = xdg.get_config_home();
    debug!("config_home: {:?}", config_home);
    watcher.watch(config_home.join("panorama"), RecursiveMode::Recursive).context("could not watch config_home")?;

    Ok((watcher, rx))
}

async fn read_config(path: impl AsRef<Path>) -> Result<MailConfig> {
    let mut file = File::open(path.as_ref())?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let config = toml::from_slice(&contents)?;
    Ok(config)
}

async fn watcher_loop(
    fs_events: Receiver<DebouncedEvent>,
    config_tx: watch::Sender<Option<MailConfig>>,
) -> Result<()> {
    // first try opening the config file directly on load
    // (so the config isn't blank until the user touches the config file)
    let xdg = BaseDirectories::new()?;
    if let Some(config_path) = xdg.find_config_file("panorama/panorama.toml") {
        debug!("found config at {:?}", config_path);
        let config = read_config(config_path).await?;
        config_tx.send(Some(config))?;
    }

    for event in fs_events {
        debug!("new event: {:?}", event);
        // config_tx.send(Some(config))?;
    }

    Ok(())
}

pub fn spawn_config_watcher() -> Result<ConfigWatcher> {
    let (_watcher, config_rx) = start_watcher()?;
    let (config_tx, config_update) = watch::channel(None);

    tokio::spawn(async move {
        match watcher_loop(config_rx, config_tx).await {
            Ok(_) => {}
            Err(err) => {
                debug!("config watcher died: {:?}", err);
            }
        }
    });

    Ok(config_update)
}
