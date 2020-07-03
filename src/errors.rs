use derive_more::Display;

#[derive(Debug, Display)]
/// Hackagotchi's backend API was unable to service you, for any of these reasons.
pub enum ServiceError {
    #[display(fmt = "Internal Server Error")]
    /// Something went wrong on our end.
    InternalServerError,

    #[display(fmt = "Bad Request: {}", _0)]
    /// The request you send us was invalid or not usable for any number of reasons.
    BadRequest(String),

    #[display(fmt = "Unauthorized")]
    /// You aren't allowed to do that.
    Unauthorized,

    #[display(fmt = "No data found")]
    /// We don't know anything about what you requested.
    NoData,
}
impl ServiceError {
    /// A shortcut for making a `ServiceError::BadRequest`.
    /// ```
    /// use hcor::ServiceError;
    ///
    /// let br = ServiceError::bad_request("you're bad and you should feel bad");
    /// let is_br = match br {
    ///     ServiceError::BadRequest(_) => true,
    ///     _ => false,
    /// };
    /// assert!(is_br, "ServiceError::bad_request() should always return a BadRequest variant");
    /// ```
    pub fn bad_request<T: ToString>(t: T) -> Self {
        Self::BadRequest(t.to_string())
    }
}

#[cfg(feature = "mongo")]
mod request_err {
    use super::*;

    #[derive(Display)]
    /// A Request was unable to be fulfilled, for a specific reason.
    /// For use inside of `ServiceError::BadRequest`s.
    pub enum RequestError {
        #[display(fmt = "Couldn't serialize: {}", _0)]
        /// We had issues saving data for later.
        Serialization(bson::ser::Error),

        #[display(fmt = "Expected Document, found: {}", _0)]
        /// We were trying to serialize some data, but it didn't turn into what we expected.
        NotDocument(bson::Bson),
    }

    impl From<bson::ser::Error> for RequestError {
        fn from(o: bson::ser::Error) -> Self {
            RequestError::Serialization(o)
        }
    }

    impl From<RequestError> for ServiceError {
        fn from(o: RequestError) -> Self {
            ServiceError::BadRequest(o.to_string())
        }
    }
}
#[cfg(feature = "mongo")]
pub use request_err::RequestError;

#[cfg(feature = "actix")]
mod actix_err {
    use super::*;
    use actix_web::{error::ResponseError, HttpResponse};

    impl ResponseError for ServiceError {
        fn error_response(&self) -> HttpResponse {
            match self {
                ServiceError::InternalServerError => HttpResponse::InternalServerError()
                    .body("Internal Server Error. Try again later."),
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
mod mongo_err {
    use super::*;
    use mongodb::error::Error as MongoError;

    impl From<MongoError> for ServiceError {
        fn from(_: MongoError) -> ServiceError {
            ServiceError::InternalServerError
        }
    }
}
