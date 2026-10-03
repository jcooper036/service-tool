use anyhow::Result;
use rusqlite::Connection;

use crate::db;
use crate::home::Home;

pub struct Ctx {
    pub home: Home,
    pub conn: Connection,
}

impl Ctx {
    pub fn open(home: Home) -> Result<Self> {
        home.ensure()?;
        let conn = db::open(&home.db_path())?;
        Ok(Self { home, conn })
    }
}
