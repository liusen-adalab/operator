use std::{path::PathBuf, sync::OnceLock};

use anyhow::Result;
use diesel::{
    query_builder::{AstPass, Query, QueryFragment, QueryId},
    r2d2::{ConnectionManager, Pool, PooledConnection},
    sql_types::BigInt,
    sqlite::Sqlite,
    QueryResult, RunQueryDsl, SqliteConnection,
};
use serde::{Deserialize, Serialize};

use crate::settings::get_settings;

pub mod application;
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

pub trait Paginate: Sized {
    fn paginate<T, T2>(self, offset: T, limit: T2) -> Paginated<Self>
    where
        i64: From<T>,
        i64: From<T2>;
}

impl<T> Paginate for T {
    fn paginate<T1, T2>(self, offset: T1, limit: T2) -> Paginated<Self>
    where
        i64: From<T1>,
        i64: From<T2>,
    {
        let offset = i64::from(offset);
        let limit = i64::from(limit);
        Paginated {
            query: self,
            offset,
            limit,
        }
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    offset: i64,
    limit: i64,
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<SqliteConnection> for Paginated<T> {}

impl<T> QueryFragment<Sqlite> for Paginated<T>
where
    T: QueryFragment<Sqlite>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.limit)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct PageList<T> {
    pub total: i64,
    pub data: Vec<T>,
}

impl<T> From<Vec<(T, i64)>> for PageList<T> {
    fn from(value: Vec<(T, i64)>) -> Self {
        let total = value.get(0).map(|(_, total)| *total).unwrap_or_default();
        let data = value.into_iter().map(|(data, _)| data).collect();
        Self { total, data }
    }
}

impl<T> PageList<T> {
    pub fn try_convert<O>(self) -> Result<PageList<O>>
    where
        T: TryInto<O, Error = anyhow::Error>,
    {
        let data = self.data.into_iter().map(|data| data.try_into()).collect::<Result<Vec<_>>>()?;
        Ok(PageList { total: self.total, data })
    }
}
