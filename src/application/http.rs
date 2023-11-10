use actix_web::web::{self, Json};

use crate::{
    http::{ApiResponse, ApiResult, Pagination},
    repositry::{self, PageList},
};

use super::{create_app as create_app_inner, Application, CreateAppParams};

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        web::scope("/api/operator")
            .route("app_list", web::post().to(app_list))
            .route("create_app", web::get().to(create_app)),
    );
}

async fn create_app(params: Json<CreateAppParams>) -> ApiResult<()> {
    create_app_inner(params.into_inner()).await?;
    ApiResponse::ok(())
}

pub async fn app_list(params: Json<Pagination>) -> ApiResult<PageList<Application>> {
    let page = params.into_inner();
    let conn = &mut repositry::db_conn().await?;
    let apps = repositry::application::list(page, conn).await?;

    ApiResponse::ok(apps)
}
