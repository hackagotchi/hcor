#[cfg(feature = "client")]
mod backend_err {
    /// Something went wrong while trying to fetch some information from a Hackagotchi backend.
    pub enum BackendError {
        Deserialization(serde_json::Error),
        HttpRequest(reqwest::Error),
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
