use crate::wormhole::Ask;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

lazy_static::lazy_static! {
    pub static ref SERVER_URL: &'static str = "http://127.0.0.1:8000";
}

/// An HTTP client, configured just the way hcor likes it :D
pub fn client() -> awc::Client {
    awc::Client::new()
}

pub type ClientResult<T> = Result<T, ClientError>;

/// Something went wrong while trying to fetch some information from a Hackagotchi backend.
#[derive(Debug)]
pub enum ClientErrorKind {
    JsonPayload(awc::error::JsonPayloadError),
    Payload(awc::error::PayloadError),
    SendRequest(awc::error::SendRequestError),
    ReturnedError(awc::http::StatusCode, String),
    BadAsk(&'static str, String),
    Wormhole(crate::wormhole::WormholeError),
    UnknownServerResponse,
}
#[derive(Debug)]
pub struct ClientError {
    route: &'static str,
    input: String,
    kind: ClientErrorKind,
}
impl ClientError {
    pub fn bad_ask(input: Ask, what: &'static str, err: impl AsRef<str>) -> Self {
        ClientError {
            route: "wormhole",
            input: format!("{:#?}", input),
            kind: ClientErrorKind::BadAsk(what, err.as_ref().to_string()),
        }
    }
}
impl std::error::Error for ClientError {}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ClientErrorKind::*;

        write!(f, "route: \"{}\"\n", self.route)?;
        write!(f, "input: \"{}\"\n", self.input)?;

        match &self.kind {
            JsonPayload(e) => write!(f, "couldn't parse JSON the server returned: {}", e),
            Payload(e) => write!(f, "couldn't parse data server returned: {}", e),
            SendRequest(e) => write!(f, "couldn't send a request to the server: {}", e),
            Wormhole(e) => write!(f, "error communicating with server through wormhole: {}", e),
            BadAsk(w, e) => write!(f, "request {} sent to wormhole returned: {}", w, e),
            ReturnedError(status, e) => {
                write!(f, "server returned Status {}, error body: {}", status, e)
            }
            UnknownServerResponse => write!(f, "server returned non-utf8 error."),
        }
    }
}
impl From<awc::error::SendRequestError> for ClientErrorKind {
    fn from(e: awc::error::SendRequestError) -> ClientErrorKind {
        ClientErrorKind::SendRequest(e)
    }
}
impl From<awc::error::JsonPayloadError> for ClientErrorKind {
    fn from(e: awc::error::JsonPayloadError) -> ClientErrorKind {
        ClientErrorKind::JsonPayload(e)
    }
}
impl From<awc::error::PayloadError> for ClientErrorKind {
    fn from(e: awc::error::PayloadError) -> ClientErrorKind {
        ClientErrorKind::Payload(e)
    }
}
impl From<crate::wormhole::WormholeError> for ClientError {
    fn from(e: crate::wormhole::WormholeError) -> ClientError {
        ClientError {
            route: "/wormhole",
            input: "unknown".to_string(),
            kind: ClientErrorKind::Wormhole(e),
        }
    }
}

/// what comes after http://127.0.0.1:8000/
pub async fn request<D: DeserializeOwned, S: Serialize + fmt::Debug>(
    endpoint: &'static str,
    input: &S,
) -> ClientResult<D> {
    let err = |kind: ClientErrorKind| ClientError {
        route: endpoint,
        input: format!("{:#?}", input),
        kind,
    };

    let mut res = client()
        .post(&format!("{}/api/{}", *SERVER_URL, endpoint))
        .send_json(&input)
        .await
        .map_err(|e| err(e.into()))?;

    let s = res.status();
    if s.is_success() {
        res.json::<D>().await.map_err(|e| err(e.into()))
    } else {
        let kind = match res.body().await {
            Ok(body) => {
                use ClientErrorKind::*;

                String::from_utf8(body.to_vec())
                    .map(|text| ReturnedError(s, text))
                    .unwrap_or(UnknownServerResponse)
            }
            Err(e) => e.into(),
        };

        Err(err(kind))
    }
}
