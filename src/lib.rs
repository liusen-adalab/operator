use std::fs;

use actix_web::{dev::Server, web, App, HttpServer};
use anyhow::Context;
use tracing::info;

use crate::settings::get_settings;

pub mod host;
pub mod http;
pub mod repositry;
pub mod settings;

pub async fn init_global() -> anyhow::Result<()> {
    settings::load_settings().context("load settings")?;
    utils::logger::init(&get_settings().log).context("init logger")?;
    init_work_dir()?;

    Ok(())
}

fn init_work_dir() -> anyhow::Result<()> {
    let settings = get_settings();
    fs::create_dir_all(&settings.data_dir).context("create data dir")?;
    let envoy_path = settings.data_dir.join(&settings.envoy.bin_path);
    let envoy_dir = envoy_path.parent().unwrap();
    fs::create_dir_all(envoy_dir).context("create envoy dir")?;

    host::init_dirs()?;

    Ok(())
}

pub fn http_server() -> anyhow::Result<Server> {
    let settings = &get_settings().http_server;
    info!(?settings, "building http server. Powered by actix-web!");

    let server: Server = HttpServer::new(move || {
        App::new()
            .configure(host::config)
            .route("/ping", web::get().to(|| async { "pong" }))
    })
    .bind((&*settings.bind, settings.port))?
    .run();

    Ok(server)
}
