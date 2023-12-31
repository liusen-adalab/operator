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
        settings.data_dir.envoy_bin_path()
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
        let mut dir = get_settings().data_dir.ssh_key_dir();
        dir.push(format!("id_{}", self.ip));
        dir
    }

    fn ssh_key_tmp_path(&self) -> PathBuf {
        let mut dir = get_settings().data_dir.ssh_key_tmp_dir();
        dir.push(format!("id_{}", self.ip));
        dir
    }

    fn ssh_global_key_path() -> PathBuf {
        get_settings().data_dir.ssh_global_dir().join("id_rsa")
    }
}

pub fn init_dirs() -> Result<()> {
    use std::fs;
    fs::create_dir_all(get_settings().data_dir.ssh_key_dir()).context("create ssh key dir")?;
    fs::create_dir_all(get_settings().data_dir.ssh_key_tmp_dir()).context("create key tmp dir")?;
    fs::create_dir_all(get_settings().data_dir.ssh_global_dir()).context("create ssh global dir")?;
    Ok(())
}
