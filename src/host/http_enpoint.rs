use std::net::IpAddr;

use actix_web::web::{self, Json, Query};
use tracing::debug;

use crate::{
    http::{ApiResponse, ApiResult, Pagination},
    repositry::{self, host, PageList},
};

use super::{ssh::HostBuilder, Host, HostId};

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        web::scope("/api/operator")
            .route("ping_host", web::get().to(ping_host))
            .route("hosts", web::post().to(host_list))
            .route("create_host", web::post().to(create_host)),
    );
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHostParams {
    pub name: String,
    pub ip: IpAddr,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
}

pub async fn create_host(params: Json<CreateHostParams>) -> ApiResult<HostId> {
    let CreateHostParams { name, ip, port, user, key } = params.into_inner();

    let mut builder = HostBuilder::new(name, ip);
    if let Some(port) = port {
        builder = builder.port(port);
    }
    if let Some(user) = user {
        builder = builder.user(user.into());
    }
    if let Some(key) = key {
        builder = builder.key(key);
    }

    let mut host = builder.build().await?;
    host.ping().await;

    let conn = &mut repositry::db_conn().await?;
    repositry::host::save(&host, conn).await?;

    ApiResponse::ok(host.id)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostIdParams {
    id: HostId,
}

pub async fn ping_host(params: Query<HostIdParams>) -> ApiResult<()> {
    let HostIdParams { id } = params.into_inner();
    debug!(?id, "ping host");
    let conn = &mut repositry::db_conn().await?;
    let mut host = repositry::host::get(id, conn)
        .await?
        .ok_or_else(|| anyhow::anyhow!("host not found"))?;

    host.ping().await;
    host::update(&host, conn).await?;
    ApiResponse::ok(())
}

pub async fn host_list(params: Json<Pagination>) -> ApiResult<PageList<Host>> {
    let page = params.into_inner();
    let conn = &mut repositry::db_conn().await?;
    let mut hosts = repositry::host::list(page, conn).await?;
    for host in hosts.data.iter_mut() {
        host.ping().await;
    }

    ApiResponse::ok(hosts)
}
