use anyhow::Result;
use std::{borrow::Cow, net::Ipv4Addr};

use crate::{
    host::{Host, HostId},
    http::Pagination,
    schema::hosts,
};
use diesel::prelude::*;

use super::SqliteConn;

#[derive(Queryable, Selectable, Identifiable, Debug, Insertable, AsChangeset)]
#[diesel(table_name = hosts)]
pub struct HostPo<'a> {
    pub id: HostId,
    pub name: Cow<'a, str>,
    pub ip: Cow<'a, str>,
}

pub async fn save(host: &Host, conn: &mut SqliteConn) -> Result<()> {
    let host = HostPo::from(host);
    diesel::insert_into(hosts::table).values(host).execute(conn)?;
    Ok(())
}

pub async fn update(host: &Host, conn: &mut SqliteConn) -> Result<()> {
    let host = HostPo::from(host);
    diesel::update(hosts::table).filter(hosts::id.eq(host.id)).set(host).execute(conn)?;
    Ok(())
}

#[derive(derive_more::From)]
pub enum HostIdent {
    Id(HostId),
    Ip(Ipv4Addr),
}

pub async fn get<T>(id: T, conn: &mut SqliteConn) -> Result<Option<Host>>
where
    HostIdent: From<T>,
{
    let id = HostIdent::from(id);
    match id {
        HostIdent::Id(id) => {
            let host: HostPo = hosts::table.select(HostPo::as_select()).find(id).first(conn)?;
            Ok(Some(Host::try_from(host)?))
        }
        HostIdent::Ip(ip) => {
            let host: HostPo = hosts::table
                .select(HostPo::as_select())
                .filter(hosts::ip.eq(ip.to_string()))
                .first(conn)?;
            Ok(Some(Host::try_from(host)?))
        }
    }
}

pub async fn list(page: Pagination, conn: &mut SqliteConn) -> Result<Vec<Host>> {
    let hosts: Vec<HostPo> = hosts::table
        .select(HostPo::as_select())
        .limit(page.limit() as i64)
        .offset(page.offset() as i64)
        .load(conn)?;

    Ok(hosts.into_iter().map(Host::try_from).collect::<Result<Vec<_>>>()?)
}
