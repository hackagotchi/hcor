use actix_web::{error::ResponseError, HttpResponse};
use derive_more::Display;
use std::convert::From;

#[derive(Debug, Display)]
pub enum ServiceError {
   
    #[display(fmt="Internal Server Error")]
    InternalServerError,

    #[display(fmt="Bad Request: {}", _0)]
    BadRequest(String),

    #[display(fmt="Unauthorized")]
    Unauthorized,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
           ServiceError::InternalServerError => HttpResponse::InternalServerError().body("Internal Server Error. Try again later."),
           ServiceError::BadRequest(s) => HttpResponse::BadRequest().body(s),
           ServiceError::Unauthorized => HttpResponse::Unauthorized().body("Unauthorized"),
        }
    }
}
