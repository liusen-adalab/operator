use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{ensure, Context};
use config::Config;
use serde::Deserialize;

use crate::repositry::SqlitePoolConfig;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub data_dir: DataDir,

    pub log: utils::logger::Config,
    pub envoy: EnvoyCfg,
    pub http_server: HttpServerCfg,
    pub sqlite: SqlitePoolConfig,
}

#[derive(Deserialize, Debug, derive_more::Deref, derive_more::AsRef)]
pub struct DataDir(PathBuf);

impl DataDir {
    pub fn sqlite_path(&self) -> PathBuf {
        self.0.join("sqlite.db")
    }

    pub fn envoy_dir(&self) -> PathBuf {
        self.0.join("bin")
    }

    pub fn envoy_bin_path(&self) -> PathBuf {
        self.envoy_dir().join("av1-envoy")
    }

    pub fn ssh_global_dir(&self) -> PathBuf {
        let mut ssh = self.0.join("ssh");
        ssh.push("global");
        ssh
    }

    pub fn ssh_key_dir(&self) -> PathBuf {
        let mut ssh = self.0.join("ssh");
        ssh.push("keys");
        ssh
    }

    pub fn ssh_key_tmp_dir(&self) -> PathBuf {
        let mut ssh = self.0.join("ssh");
        ssh.push("tmp");
        ssh
    }
}

#[derive(Deserialize, Debug)]
pub struct EnvoyCfg {
    pub bin_path: PathBuf,
    pub port: u16,
}

#[derive(Deserialize, Debug)]
pub struct HttpServerCfg {
    pub bind: String,
    pub port: u16,
}

#[macro_export]
macro_rules! join_path {
    ($pre:expr, $child:expr) => {{
        let child = if $child.starts_with("/") {
            $child.strip_prefix("/").unwrap()
        } else {
            $child
        };
        $pre.join(child)
    }};
}

pub fn get_settings() -> &'static Settings {
    unsafe { SETTINGS.get().unwrap_unchecked() }
}

static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[cfg(not(debug_assertions))]
pub static CONFIG_DIR: &str = "/etc/av1-operator/configs";
#[cfg(debug_assertions)]
pub static CONFIG_DIR: &str = "./configs";

pub fn load_settings() -> anyhow::Result<&'static Settings> {
    let config_dir = Path::new(CONFIG_DIR);

    let default = config::File::from(config_dir.join("default.toml")).required(false);
    let beta = config::File::from(config_dir.join("beta.toml")).required(false);
    let release = config::File::from(config_dir.join("release.toml")).required(false);
    let mut builder = Config::builder().add_source(default).add_source(beta).add_source(release);
    builder = builder;

    let settings: Settings = builder
        .build()
        .context("cannot load config")?
        .try_deserialize()
        .context("wrong config format")?;

    ensure!(settings.data_dir.is_absolute(), "data_dir must be absolute path");

    Ok(SETTINGS.get_or_init(|| settings))
}
