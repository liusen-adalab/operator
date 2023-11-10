use std::path::PathBuf;

use serde::Serialize;
use utils::id_new_type;

pub mod convert;
pub mod http;

id_new_type!(AppId);

#[derive(Serialize)]
pub struct Application {
    id: AppId,
    name: String,
    git: String,
    versions: Vec<AppVersioned>,
}

#[derive(Serialize)]
struct AppVersioned {
    hash: String,
}

pub use create::*;
mod create {
    use anyhow::Result;
    use tokio::{
        fs::{self, File},
        io::AsyncWriteExt,
    };
    use utils::async_cmd;

    use crate::{
        application::{AppId, Application},
        repositry,
        settings::get_settings,
    };

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateAppParams {
        pub name: String,
        pub git: String,
        pub scripts: AppScripts,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AppScripts {
        pub build: String,
        pub install: String,
    }

    pub async fn create_app(params: CreateAppParams) -> Result<()> {
        let CreateAppParams { name, git, scripts } = params;
        let app_dir = get_settings().data_dir.app_dir(&name);
        fs::create_dir_all(&*app_dir).await?;

        let code_dir = app_dir.code_dir();
        async_cmd!("git", "clone", git, code_dir);

        let mut file = File::options().create(true).write(true).open(app_dir.join("build.sh")).await?;
        file.write_all(scripts.build.as_bytes()).await?;
        let mut file = File::options().create(true).write(true).open(app_dir.join("install.sh")).await?;
        file.write_all(scripts.install.as_bytes()).await?;

        let app = Application {
            id: AppId::next_id(),
            name,
            git,
            versions: Default::default(),
        };

        let conn = &mut repositry::db_conn().await?;
        repositry::application::save(&app, conn).await?;

        Ok(())
    }
}

#[derive(Debug, derive_more::Deref, derive_more::AsRef, derive_more::From)]
pub struct AppDir(PathBuf);

impl AppDir {
    fn code_dir(&self) -> PathBuf {
        self.0.join("code")
    }

    fn build_out_dir(&self) -> PathBuf {
        self.0.join("build-out")
    }

    fn build_script_path(&self) -> PathBuf {
        self.0.join("build.sh")
    }

    fn install_script_path(&self) -> PathBuf {
        self.0.join("install.sh")
    }
}

mod build {
    use std::{os::fd::FromRawFd, process::Stdio};

    use anyhow::{Context, Result};
    use serde::Deserialize;
    use tokio::io::AsyncReadExt;
    use utils::{async_cmd, macros::async_cmd::async_process::Command};

    use crate::{
        application::AppVersioned,
        repositry::{self, application::AppVersionPo},
        settings::get_settings,
    };

    use super::{AppId, Application};

    #[derive(Deserialize)]
    struct BuildAppParams {
        app_id: AppId,
        hash: String,
        build_script: Option<String>,
    }

    pub async fn build_app(params: BuildAppParams) -> Result<()> {
        let conn = &mut repositry::db_conn().await?;
        let app = repositry::application::find(params.app_id, conn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("app not found"))?;

        todo!()
    }

    impl Application {
        async fn build(&self, hash: &str, script: Option<String>) -> Result<String> {
            let app_dir = get_settings().data_dir.app_dir(&self.name);
            let code_dir = app_dir.code_dir();

            async_cmd!(pwd = &code_dir; "git", "checkout", hash, code_dir);
            let out = if let Some(script) = script {
                async_cmd!(pwd = &code_dir; "bash", "-c", script)
            } else {
                let build_script_path = app_dir.build_script_path();
                let mut file = tokio::fs::File::open(&build_script_path).await.context("open build script")?;
                let mut script = String::new();
                file.read_to_string(&mut script).await.context("read build script")?;
                async_cmd!(pwd = &code_dir; "bash", "-c", script)
            };

            let out = String::from_utf8(out.stdout).context("output is not utf-8")?;

            let conn = &mut repositry::db_conn().await?;
            let version = AppVersionPo {
                hash: hash.into(),
                app_id: self.id,
            };
            repositry::application::new_version(version, conn).await?;

            Ok(out)
        }
    }

    #[tokio::test]
    async fn t_bash() -> Result<()> {
        let cwd = &std::env::current_dir().unwrap();
        let script = r#"
            set -e
            set -x

            echo "hello"
            echo "world"
            # ls -al
        "#;
        let cmd = Command::new("program");

        let out = async_cmd!(pwd = cwd; "bash", "-c", script);
        println!("{}", String::from_utf8(out.stdout).unwrap());
        println!("{}", String::from_utf8(out.stderr).unwrap());
        Ok(())
    }
}
