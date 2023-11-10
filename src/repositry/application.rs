use std::{borrow::Cow, collections::HashMap};

use crate::{
    application::{AppId, Application},
    http::Pagination,
    schema::{app_versions, applications},
};
use anyhow::Result;
use diesel::prelude::*;
use diesel::result::OptionalExtension;

use super::{PageList, Paginate, SqliteConn};

#[derive(Queryable, Selectable, Identifiable, Debug, Insertable, AsChangeset)]
#[diesel(table_name = applications)]
pub struct ApplicaionPo<'a> {
    pub id: AppId,
    pub name: Cow<'a, str>,
    pub git_url: Cow<'a, str>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Insertable)]
#[diesel(table_name = app_versions)]
#[diesel(primary_key(hash))]
pub struct AppVersionPo<'a> {
    pub hash: Cow<'a, str>,
    pub app_id: AppId,
}

pub async fn save(app: &Application, conn: &mut SqliteConn) -> Result<()> {
    let app = ApplicaionPo::from(app);
    diesel::insert_into(applications::table).values(app).execute(conn)?;
    Ok(())
}

pub async fn new_version(version: AppVersionPo<'_>, conn: &mut SqliteConn) -> Result<()> {
    diesel::insert_into(app_versions::table).values(version).execute(conn)?;
    Ok(())
}

pub async fn find(id: AppId, conn: &mut SqliteConn) -> Result<Option<Application>> {
    let app = applications::table
        .select(ApplicaionPo::as_select())
        .find(id)
        .first(conn)
        .optional()?;
    let Some(app) = app else { return Ok(None) };

    let versions: Vec<AppVersionPo> = app_versions::table
        .select(AppVersionPo::as_select())
        .filter(app_versions::app_id.eq(id))
        .load(conn)?;

    let app = Application::try_from((app, versions))?;
    Ok(Some(app))
}

pub async fn list(page: Pagination, conn: &mut SqliteConn) -> Result<PageList<Application>> {
    let apps: Vec<(ApplicaionPo, i64)> = applications::table
        .select(ApplicaionPo::as_select())
        .paginate(page.offset(), page.limit())
        .load(conn)?;

    let app_ids = apps.iter().map(|(app, _)| app.id).collect::<Vec<_>>();
    let app_list = PageList::from(apps);
    let mut versions: Vec<AppVersionPo> = app_versions::table
        .select(AppVersionPo::as_select())
        .filter(app_versions::app_id.eq_any(app_ids))
        .load(conn)?;

    let mut app_groups = HashMap::new();
    while let Some(version) = versions.pop() {
        app_groups.entry(version.app_id).or_insert_with(Vec::new).push(version);
    }
    let apps = app_list
        .data
        .into_iter()
        .map(|app| {
            let versions = app_groups.remove(&app.id).unwrap_or_default();
            Application::try_from((app, versions))
        })
        .collect::<Result<Vec<_>>>();

    Ok(PageList {
        data: apps?,
        total: app_list.total,
    })
}
