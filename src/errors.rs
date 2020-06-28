use derive_more::Display;
use std::convert::From;

#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,

    #[display(fmt = "Bad Request: {}", _0)]
    BadRequest(String),

    #[display(fmt = "Unauthorized")]
    Unauthorized,

    #[display(fmt = "No data found")]
    NoData,
}

#[cfg(feature = "actix")]
mod actix {
    use super::*;
    use actix_web::{error::ResponseError, HttpResponse};

    impl ResponseError for ServiceError {
        fn error_response(&self) -> HttpResponse {
            match self {
                ServiceError::InternalServerError => {
                    HttpResponse::InternalServerError().body("Internal Server Error. Try again later.")
                }
                ServiceError::BadRequest(s) => HttpResponse::BadRequest().body(s),
                ServiceError::Unauthorized => HttpResponse::Unauthorized().body("Unauthorized"),
                ServiceError::NoData => HttpResponse::NotFound().body("Data not found"),
            }
        }
    }
}

#[cfg(feature = "actix")]
impl From<Box<dyn std::error::Error>> for ServiceError {
    fn from(_: Box<dyn std::error::Error>) -> ServiceError {
        ServiceError::InternalServerError
    }
}

#[cfg(feature = "mongo")]
mod mongo {
    use super::*;
    use mongodb::error::Error as MongoError;

    impl From<MongoError> for ServiceError {
        fn from(_: MongoError) -> ServiceError {
            ServiceError::InternalServerError
        }
    }
}
