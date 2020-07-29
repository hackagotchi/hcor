#[cfg(feature = "client")]
mod backend_err {
    pub use std::fmt;

    /// Something went wrong while trying to fetch some information from a Hackagotchi backend.
    #[derive(Debug)]
    pub enum BackendError {
        Deserialization(serde_json::Error),
        HttpRequest(reqwest::Error),
    }
    impl std::error::Error for BackendError {}
    impl fmt::Display for BackendError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            use BackendError::*;

            match self {
                Deserialization(e) => write!(f, "couldn't parse what server returned: {}", e),
                HttpRequest(e) => write!(f, "server returned error: {}", e),
            }
        }
    }
    impl From<serde_json::Error> for BackendError {
        fn from(e: serde_json::Error) -> BackendError {
            BackendError::Deserialization(e)
        }
    }
    impl From<reqwest::Error> for BackendError {
        fn from(e: reqwest::Error) -> BackendError {
            BackendError::HttpRequest(e)
        }
    }
}
#[cfg(feature = "client")]
pub use backend_err::BackendError;
