use std::sync::OnceLock;

use crate::{
    host::{Host, HostId},
    http::Pagination,
    join_path,
    settings::get_settings,
};
use anyhow::{Context, Result};

use super::IdToBytes;

impl IdToBytes for HostId {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}

pub async fn save(host: &Host) -> Result<()> {
    let bytes = bincode::serialize(&host)?;
    db().insert(host.id.to_bytes(), bytes)?;
    Ok(())
}

pub async fn get(id: HostId) -> Result<Option<Host>> {
    let bytes = db().get(id.to_bytes()).context("query db")?;
    let host = match bytes {
        Some(bytes) => bincode::deserialize(&bytes).context("deserialize host")?,
        None => return Ok(None),
    };
    Ok(Some(host))
}

pub async fn list(page: Pagination) -> Result<Vec<Host>> {
    let mut hosts = Vec::new();
    for kv in db().iter().skip(page.offset()).take(page.limit()) {
        let (_key, value) = kv?;
        let host = bincode::deserialize(&value)?;
        hosts.push(host);
    }
    Ok(hosts)
}

static DB: OnceLock<sled::Db> = OnceLock::new();

fn db() -> &'static sled::Db {
    DB.get_or_init(|| {
        let settings = get_settings();
        let path = join_path!(settings.data_dir, &settings.sled.path);
        sled::open(path).unwrap()
    })
}
