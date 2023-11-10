use std::{path::PathBuf, sync::OnceLock};

use anyhow::Result;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    SqliteConnection,
};
use serde::{Deserialize, Serialize};

use crate::settings::get_settings;

// pub mod host;
pub mod host;

#[derive(Debug, Deserialize)]
pub struct SledCfg {
    pub path: PathBuf,
}

trait IdToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}

/// 连接池配置
#[derive(Debug, Serialize, Deserialize)]
pub struct SqlitePoolConfig {
    /// 最小连接数，当连接池中存活的连接数小于这个值时，会自动建立新连接
    pub min_conn: u32,
    /// 最大连接数
    pub max_conn: u32,
}

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;
type SqliteConn = PooledConnection<ConnectionManager<SqliteConnection>>;
static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub fn init(cfg: &SqlitePoolConfig) -> Result<&'static Pool<ConnectionManager<SqliteConnection>>> {
    use diesel::prelude::*;
    let path = get_settings().data_dir.sqlite_path();
    let url = format!("sqlite://{}?mode=rwc", path.to_str().unwrap());
    let manager = ConnectionManager::<SqliteConnection>::new(url);
    let pool = Pool::builder()
        .min_idle(Some(cfg.min_conn))
        .max_size(cfg.max_conn)
        .test_on_check_out(true)
        .build(manager)?;
    Ok(POOL.get_or_init(|| pool))
}

pub async fn db_conn() -> Result<SqliteConn> {
    tokio::task::spawn_blocking(db_conn_sync).await?
}

pub fn db_conn_sync() -> Result<SqliteConn> {
    let pool = POOL.get().unwrap();
    let conn = pool.get()?;
    Ok(conn)
}
