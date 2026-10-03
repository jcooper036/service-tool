use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct Home {
    pub root: PathBuf,
}

impl Home {
    pub fn resolve(override_root: Option<PathBuf>) -> Result<Self> {
        let root = match override_root {
            Some(root) => root,
            None => dirs::home_dir().context("no home directory")?.join(".ser"),
        };
        Ok(Self { root })
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn db_path(&self) -> PathBuf {
        self.root.join("ser.db")
    }

    pub fn ensure(&self) -> Result<()> {
        let dir = self.logs_dir();
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))
    }
}
