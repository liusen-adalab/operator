use std::net::{IpAddr, SocketAddr};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;
use utils::id_new_type;
use volo_gen::av1::operator::{NodeServiceClient, NodeServiceClientBuilder, Ping};

use crate::{host::builder::HostBuilder, settings::get_settings};

id_new_type!(HostId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub name: String,
    pub ip: IpAddr,
    pub state: HostState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostState {
    Running,
    Stopped,
    Disconnected,
}

impl Host {
    pub async fn ping(&mut self) {
        let client = self.client();
        let pong = client.ping(Ping { message: "ping".into() }).await;
        if pong.is_err() {
            debug!(?pong, "ping host error");
            self.state = HostState::Disconnected;
        } else {
            self.state = HostState::Running;
        }

        debug!(?pong);
    }

    fn client(&self) -> NodeServiceClient {
        let port = get_settings().envoy.port;
        let addr = SocketAddr::new(self.ip, port);
        debug!(?addr, "create grpc client");
        let client = NodeServiceClientBuilder::new("av1-operator").address(addr).build();
        client
    }
}

pub fn init_dirs() -> Result<()> {
    use std::fs;
    fs::create_dir_all(HostBuilder::ssh_key_dir()).context("create ssh key dir")?;
    fs::create_dir_all(HostBuilder::ssh_key_tmp_dir()).context("create key tmp dir")?;
    fs::create_dir_all(HostBuilder::ssh_global_dir()).context("create ssh global dir")?;
    Ok(())
}

pub mod builder {
    use std::{
        net::IpAddr,
        path::{Path, PathBuf},
    };

    use anyhow::{Context, Result};
    use tokio::{
        fs::{self, File},
        io::AsyncWriteExt,
    };
    use tracing::debug;
    use utils::async_cmd;
    use volo::FastStr;

    use crate::settings::{get_settings, CONFIG_DIR};

    use super::{Host, HostId};

    pub struct HostBuilder {
        name: String,
        ip: IpAddr,
        port: u16,
        user: FastStr,
        key: Option<String>,
    }

    impl HostBuilder {
        pub fn new(name: String, ip: IpAddr) -> Self {
            Self {
                name,
                ip,
                port: 22,
                user: "root".into(),
                key: None,
            }
        }

        pub fn port(mut self, port: u16) -> Self {
            self.port = port;
            self
        }

        pub fn user(mut self, user: FastStr) -> Self {
            self.user = user;
            self
        }

        pub fn key(mut self, key: String) -> Self {
            self.key = Some(key);
            self
        }

        pub async fn build(self) -> Result<Host> {
            self.ssh_auth(self.key.as_deref()).await?;
            self.send_envoy().await?;

            Ok(Host {
                id: HostId::next_id(),
                ip: self.ip,
                state: super::HostState::Running,
                name: self.name,
            })
        }

        async fn send_envoy(&self) -> Result<()> {
            // open port
            let port = get_settings().envoy.port;
            self.run_ssh_cmd(&format!("firewall-cmd --add-port {}/tcp", port)).await?;

            // stop envoy
            self.run_ssh_cmd("systemctl stop av1-envoy || true").await?;

            // sync app binary
            let app_path = Self::envoy_bin_path();
            self.scp(&app_path, Path::new("/usr/local/bin/av1-envoy")).await?;
            // sync systemd config
            let service_path = Path::new(CONFIG_DIR).join("av1-envoy.service");
            self.scp(&service_path, Path::new("/etc/systemd/system/av1-envoy.service")).await?;

            // start
            self.run_ssh_cmd("systemctl daemon-reload").await?;
            self.run_ssh_cmd("systemctl start av1-envoy.service").await?;
            self.run_ssh_cmd("systemctl enable av1-envoy.service").await?;

            Ok(())
        }

        fn envoy_bin_path() -> PathBuf {
            let settings = &get_settings();
            settings.data_dir.join(settings.envoy.bin_path.as_path())
        }

        async fn ssh_auth(&self, key: Option<&str>) -> Result<()> {
            // save key to tmp file
            let tmp_path = match key {
                Some(key) => {
                    let tmp_path = self.ssh_key_tmp_path();
                    let mut file = File::options().create(true).write(true).open(&tmp_path).await?;
                    file.write_all(key.as_bytes()).await.context("error: write key to temp file")?;
                    tmp_path
                }
                None => {
                    let global_key = Self::ssh_global_key_path();
                    if !global_key.exists() {
                        anyhow::bail!("no global key");
                    }
                    global_key
                }
            };

            let conn_ok = Self::test_ssh_conn(&tmp_path, &*self.user, self.ip, self.port).await?;
            if conn_ok {
                // move key file from tmp to data
                let key_path = self.ssh_key_path();
                fs::rename(tmp_path, key_path).await?;
            }

            Ok(())
        }

        async fn test_ssh_conn(key: &Path, user: &str, ip: IpAddr, port: u16) -> Result<bool> {
            let host_addr = format!("{}@{}", user, ip);
            // TODO: 区分不同的错误，比如密钥错误，端口错误等
            let t = async {
                // ssh -q -o BatchMode=yes -o StrictHostKeyChecking=no -o ConnectTimeout=5 web15 "exit 0"
                async_cmd!(
                    "ssh",
                    "-q",
                    "-o",
                    "BatchMode=yes",
                    "-o",
                    "StrictHostKeyChecking=no",
                    "-o",
                    "ConnectTimeout=5",
                    "-p",
                    port.to_string(),
                    "-i",
                    key,
                    host_addr,
                    "exit 0"
                );
                anyhow::Ok(())
            };

            Ok(t.await.is_ok())
        }

        async fn scp(&self, src: &Path, dst: &Path) -> Result<()> {
            debug!(?src, ?dst, "scp file");
            let dst = self.ssh_url() + ":" + &*dst.to_string_lossy();
            let port = self.port.to_string();
            let identity = self.ssh_key_path();
            async_cmd!("scp", "-O", "-i", identity, "-r", "-P", port, src, dst);
            Ok(())
        }

        async fn run_ssh_cmd(&self, cmd: &str) -> Result<()> {
            debug!(cmd, "run ssh cmd");
            let port = self.port.to_string();
            let identity = self.ssh_key_path();
            async_cmd!("ssh", "-i", identity, "-p", port, self.ssh_url(), cmd);
            Ok(())
        }

        fn ssh_url(&self) -> String {
            format!("{}@{}", self.user, self.ip)
        }

        fn ssh_key_path(&self) -> PathBuf {
            if self.key.is_none() {
                return Self::ssh_global_key_path();
            }
            let mut dir = Self::ssh_key_dir();
            dir.push(format!("id_{}", self.ip));
            dir
        }

        fn ssh_key_tmp_path(&self) -> PathBuf {
            let mut dir = Self::ssh_key_tmp_dir();
            dir.push(format!("id_{}", self.ip));
            dir
        }

        fn ssh_global_key_path() -> PathBuf {
            Self::ssh_global_dir().join("id_rsa")
        }

        pub fn ssh_global_dir() -> PathBuf {
            let mut path = get_settings().data_dir.clone();
            path.push("ssh");
            path.push("global");
            path
        }

        pub fn ssh_key_dir() -> PathBuf {
            let mut path = get_settings().data_dir.clone();
            path.push("ssh");
            path.push("keys");
            path
        }

        pub fn ssh_key_tmp_dir() -> PathBuf {
            let mut path = get_settings().data_dir.clone();
            path.push("ssh");
            path.push("tmp");
            path
        }
    }
}

pub use http::*;
pub mod http {
    use std::net::IpAddr;

    use actix_web::web::{self, Json, Query};
    use tracing::debug;

    use crate::{
        http::{ApiResponse, ApiResult, Pagination},
        repositry::{self, host},
    };

    use super::{builder::HostBuilder, Host, HostId};

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

        repositry::host::save(&host).await?;

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
        let mut host = repositry::host::get(id).await?.ok_or_else(|| anyhow::anyhow!("host not found"))?;
        host.ping().await;
        host::save(&host).await?;
        ApiResponse::ok(())
    }

    pub async fn host_list(params: Json<Pagination>) -> ApiResult<Vec<Host>> {
        let page = params.into_inner();
        let hosts = repositry::host::list(page).await?;
        ApiResponse::ok(hosts)
    }
}
