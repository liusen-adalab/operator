use std::{fmt::Display, num::ParseIntError};

use actix_web::{body::BoxBody, http::StatusCode, web::Json, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    status: u32,
    err_msg: Option<String>,
    data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> ApiResult<T> {
        Ok(Json(Self {
            status: 0,
            err_msg: None,
            data: Some(data),
        }))
    }
}

#[allow(dead_code)]
impl ApiResponse<()> {
    pub fn empty_ok() -> Self {
        ApiResponse {
            status: 0,
            err_msg: None,
            data: None,
        }
    }

    pub fn simple_err<M>(msg: M) -> ApiResult<()>
    where
        M: Display + std::fmt::Debug + Send + Sync + 'static,
    {
        Err(ApiError::from(anyhow::Error::msg(msg)))
    }
}

pub type ApiResult<T> = Result<Json<ApiResponse<T>>, ApiError>;

pub trait ErrorTrait: std::fmt::Debug + Display + 'static {}
#[derive(derive_more::Display, Debug)]
#[display(fmt = "error: {msg:?}")]
pub struct ApiError {
    msg: Box<dyn ErrorTrait>,
}

impl ErrorTrait for anyhow::Error {}
impl ErrorTrait for ParseIntError {}

impl<T> From<T> for ApiError
where
    T: ErrorTrait,
{
    fn from(value: T) -> Self {
        Self { msg: Box::new(value) }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        info!(err = ?self.msg, "api error");
        let resp = ApiResponse::<()> {
            status: 1,
            err_msg: Some(self.to_string()),
            data: None,
        };
        HttpResponse::build(self.status_code()).json(resp)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    page: u16,
    page_size: u16,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page: 1, page_size: 10 }
    }
}

impl Pagination {
    pub fn offset(&self) -> usize {
        (self.page.checked_sub(1).unwrap_or_default()) as usize * self.page_size as usize
    }

    pub fn limit(&self) -> usize {
        self.page_size as usize
    }
}
