use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Context;
use config::Config;
use serde::Deserialize;

use crate::repositry::SledCfg;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub log: utils::logger::Config,
    pub data_dir: PathBuf,
    pub envoy: EnvoyCfg,
    pub http_server: HttpServerCfg,
    pub sled: SledCfg,
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

    Ok(SETTINGS.get_or_init(|| settings))
}