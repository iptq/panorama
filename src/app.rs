use std::ops::Deref;

use crate::config::ConfigWatcher;

#[async_trait]
pub trait AnyApp {
    fn say_hello(&self) {
        debug!("hello from app");
    }
}

/// An App is usually associated with a particular separate service or piece of functionality.
pub struct App<A: AppI> {
    inner: A,
    // config_watcher: ConfigWatcher<A::Config>,
}

impl<A: AppI> App<A> {
    pub fn new(app: A) -> Self {
        App { inner: app }
    }
}

/// The interface that anything that wants to become an App must implement
pub trait AppI: Sized {
    type Config;
}

impl<A: AppI> AnyApp for App<A> {}
