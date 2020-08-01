use crate::UserId;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;
use uuid::Uuid;

lazy_static::lazy_static! {
    pub static ref SERVER_URL: &'static str = "http://127.0.0.1:8000";
}

/// An HTTP client, configured just the way hcor likes it :D
fn client() -> awc::Client {
    awc::Client::new()
}

pub type ClientResult<T> = Result<T, ClientError>;

/// Something went wrong while trying to fetch some information from a Hackagotchi backend.
#[derive(Debug)]
pub enum ClientError {
    JsonPayload(awc::error::JsonPayloadError),
    Payload(awc::error::PayloadError),
    SendRequest(awc::error::SendRequestError),
    ReturnedError(awc::http::StatusCode, String),
    UnknownServerResponse,
    ExpectedOneSpawnReturnedNone,
}
impl std::error::Error for ClientError {}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ClientError::*;

        match self {
            JsonPayload(e) => write!(f, "couldn't parse JSON the server returned: {}", e),
            Payload(e) => write!(f, "couldn't parse data server returned: {}", e),
            SendRequest(e) => write!(f, "couldn't send a request to the server: {}", e),
            ReturnedError(status, e) => {
                write!(f, "server returned Status {}, error body: {}", status, e)
            }
            UnknownServerResponse => write!(f, "server returned non-utf8 error."),
            ExpectedOneSpawnReturnedNone => write!(
                f,
                "attempted to spawn a single item, but the server returned no items"
            ),
        }
    }
}
impl From<awc::error::SendRequestError> for ClientError {
    fn from(e: awc::error::SendRequestError) -> ClientError {
        ClientError::SendRequest(e)
    }
}
impl From<awc::error::JsonPayloadError> for ClientError {
    fn from(e: awc::error::JsonPayloadError) -> ClientError {
        ClientError::JsonPayload(e)
    }
}
impl From<awc::error::PayloadError> for ClientError {
    fn from(e: awc::error::PayloadError) -> ClientError {
        ClientError::Payload(e)
    }
}

/// what comes after http://127.0.0.1:8000/
pub async fn request<D: DeserializeOwned, S: Serialize>(
    endpoint: &str,
    input: &S,
) -> ClientResult<D> {
    let mut res = client()
        .post(&format!("{}/{}", *SERVER_URL, endpoint))
        .send_json(&input)
        .await?;

    let s = res.status();
    if s.is_success() {
        Ok(res.json::<D>().await?)
    } else {
        match String::from_utf8(res.body().await?.to_vec()) {
            Ok(text) => Err(ClientError::ReturnedError(s, text)),
            Err(_) => Err(ClientError::UnknownServerResponse),
        }
    }
}

pub trait IdentifiesSteader {
    fn steader_id(self) -> Uuid;
}
impl IdentifiesSteader for Uuid {
    fn steader_id(self) -> Uuid {
        self
    }
}
impl IdentifiesSteader for &Uuid {
    fn steader_id(self) -> Uuid {
        *self
    }
}
impl IdentifiesSteader for &crate::Hackstead {
    fn steader_id(self) -> Uuid {
        self.profile.steader_id
    }
}
impl IdentifiesSteader for &mut crate::Hackstead {
    fn steader_id(self) -> Uuid {
        self.profile.steader_id
    }
}
impl IdentifiesSteader for &crate::hackstead::Profile {
    fn steader_id(self) -> Uuid {
        self.steader_id
    }
}
impl IdentifiesSteader for &mut crate::hackstead::Profile {
    fn steader_id(self) -> Uuid {
        self.steader_id
    }
}

pub trait IdentifiesUser {
    fn user_id(self) -> UserId;
}
impl IdentifiesUser for &UserId {
    fn user_id(self) -> UserId {
        self.clone()
    }
}
impl<S: IdentifiesSteader> IdentifiesUser for S {
    fn user_id(self) -> UserId {
        UserId::Uuid(self.steader_id())
    }
}

pub trait IdentifiesTile {
    fn tile_id(self) -> Uuid;
}
impl IdentifiesTile for Uuid {
    fn tile_id(self) -> Uuid {
        self
    }
}
impl IdentifiesTile for &Uuid {
    fn tile_id(self) -> Uuid {
        *self
    }
}
impl IdentifiesTile for &crate::Tile {
    fn tile_id(self) -> Uuid {
        self.base.tile_id
    }
}

/// Plants are referred to as tile Ids, but through this interface,
/// Plants can refer to Tiles, but Tiles can't refer to Plants since
/// Tiles may or may not actually have plants on them.
pub trait IdentifiesPlant {
    fn tile_id(self) -> Uuid;
}
impl IdentifiesPlant for &crate::Plant {
    fn tile_id(self) -> Uuid {
        self.base.tile_id
    }
}
impl<T: IdentifiesTile> IdentifiesPlant for T {
    fn tile_id(self) -> Uuid {
        self.tile_id()
    }
}

pub trait IdentifiesItem {
    fn item_id(self) -> Uuid;
}
impl IdentifiesItem for Uuid {
    fn item_id(self) -> Uuid {
        self
    }
}
impl IdentifiesItem for &Uuid {
    fn item_id(self) -> Uuid {
        *self
    }
}
impl IdentifiesItem for &crate::Item {
    fn item_id(self) -> Uuid {
        self.base.item_id
    }
}
impl IdentifiesItem for &mut crate::Item {
    fn item_id(self) -> Uuid {
        self.base.item_id
    }
}
impl IdentifiesItem for &crate::item::ItemBase {
    fn item_id(self) -> Uuid {
        self.item_id
    }
}
impl IdentifiesItem for &mut crate::item::ItemBase {
    fn item_id(self) -> Uuid {
        self.item_id
    }
}
