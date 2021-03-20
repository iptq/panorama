use std::path::{Path, PathBuf};

use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::fs::{self, File, OpenOptions};

pub struct Maildir {
    path: PathBuf,
}

impl Maildir {
    // TODO: should this double as create (aka create tmp cur new if they don't exist)?
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Maildir {
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Stores a new message into the `new` directory
    // TODO: maybe have a streaming option?
    pub async fn store(&self, opts: StoreOptions) -> Result<PathBuf> {
        let unique_name = opts.create_unique_name();
        let tmp_file = self.tmp_dir().join(&unique_name);
        {
            let mut file = OpenOptions::new()
                .create_new(true) // fail if the file already exists, this means we aren't unique!
                .open(&tmp_file)
                .await?;
        }

        let new_file = self.new_dir().join(&unique_name);
        fs::rename(&tmp_file, &new_file).await?;
        Ok(new_file)
    }

    /// Returns the path to the `tmp` subdirectory
    #[inline]
    pub fn tmp_dir(&self) -> PathBuf {
        self.path.join("tmp")
    }

    /// Returns the path to the `new` subdirectory
    #[inline]
    pub fn new_dir(&self) -> PathBuf {
        self.path.join("new")
    }

    /// Returns the path to the `cur` subdirectory
    #[inline]
    pub fn cur_dir(&self) -> PathBuf {
        self.path.join("cur")
    }
}

/// Options that will be used to determine the filename it's stored to
pub struct StoreOptions {}

impl StoreOptions {
    pub fn create_unique_name(&self) -> String {
        format!("")
    }
}
