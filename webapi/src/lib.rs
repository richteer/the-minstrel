pub mod api;
pub mod embed;
pub mod web;
pub mod user;

use warp::http::StatusCode;
use model::web::ReplyData;
pub trait ReplyStatusFuncs {
    fn new<S: Into<String>>(status: StatusCode, error: S, data: Option<ReplyData>) -> Self;
    fn ok() -> Self;
    fn ok_data(data: ReplyData) -> Self;
    fn new_nd<S: Into<String>>(status: StatusCode, error: S) -> Self;
    fn uerr<S: Into<String>>(error: S) -> Self;
}

impl ReplyStatusFuncs for model::web::ReplyStatus {


    fn new<S: Into<String>>(status: StatusCode, error: S, data: Option<ReplyData>) -> Self {
        Self {
            status: status.try_into().unwrap(),
            error: error.into(),
            data,
        }
    }

    fn ok() -> Self {
        Self::new(StatusCode::OK, "ok", None)
    }

    fn ok_data(data: ReplyData) -> Self {
        Self::new(StatusCode::OK, "ok", Some(data))
    }

    fn new_nd<S: Into<String>>(status: StatusCode, error: S) -> Self {
        Self::new(status, error, None)
    }

    fn uerr<S: Into<String>>(error: S) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error, None)
    }
}