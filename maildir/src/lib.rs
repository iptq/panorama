use std::path::{Path, PathBuf};

use tokio::fs::{File, self};
use anyhow::Result;
use tempfile::NamedTempFile;

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
    pub async fn store(&self) -> Result<()> {
        let unique_name = "hellosu";
        let tmp_file = self.tmp_dir().join(unique_name);
        {
            let mut file = File::create(&tmp_file).await?;
        }

        let new_file = self.new_dir().join(unique_name);
        fs::rename(tmp_file, new_file).await?;
        Ok(())
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
