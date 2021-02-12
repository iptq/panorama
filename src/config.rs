use std::marker::PhantomData;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::Result;
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use xdg::BaseDirectories;

pub struct ConfigWatcher<C> {
    _ty: PhantomData<C>,
}

pub fn watch_config() -> Result<()> {
    let (tx, rx) = channel();

    let xdg = BaseDirectories::new()?;
    let config_home = xdg.get_config_home();
    let mut watcher = notify::watcher(tx, Duration::from_secs(5))?;
    watcher.watch(config_home, RecursiveMode::Recursive)?;

    loop {
        let evt = rx.recv()?;
    }
    Ok(())
}
