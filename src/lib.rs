use std::fs;

use actix_web::{dev::Server, web, App, HttpServer};
use anyhow::Context;
use tracing::info;

use crate::{repositry::db_conn, settings::get_settings};

mod host;
mod http;
mod repositry;
mod schema;
mod settings;

pub async fn init_global() -> anyhow::Result<()> {
    let settings = settings::load_settings().context("load settings")?;
    utils::logger::init(&get_settings().log).context("init logger")?;
    repositry::init(&settings.sqlite).context("init sqlite pool")?;
    init_work_dir().context("init data dir")?;

    // Run migrations
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
    let conn = &mut db_conn().await?;
    conn.run_pending_migrations(MIGRATIONS).map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

fn init_work_dir() -> anyhow::Result<()> {
    let settings = get_settings();
    let data_dir = &settings.data_dir;
    fs::create_dir_all(&**data_dir).context("create data dir")?;
    fs::create_dir_all(data_dir.envoy_dir()).context("create envoy dir")?;

    host::ssh::init_dirs()?;
    Ok(())
}

pub fn http_server() -> anyhow::Result<Server> {
    let settings = &get_settings().http_server;
    info!(?settings, "building http server. Powered by actix-web!");

    let server: Server = HttpServer::new(move || {
        App::new()
            .configure(host::http_enpoint::config)
            .route("/ping", web::get().to(|| async { "pong" }))
    })
    .bind((&*settings.bind, settings.port))?
    .run();

    Ok(server)
}
