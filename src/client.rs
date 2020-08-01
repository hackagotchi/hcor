use crate::UserId;
use std::fmt;
use uuid::Uuid;

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
pub enum ClientError {
    Deserialization(awc::error::JsonPayloadError),
    RawDeserialization(awc::error::PayloadError),
    HttpRequest(awc::error::SendRequestError),
    BadRequest(awc::http::StatusCode, String),
    ServerError(awc::http::StatusCode, String),
    UnknownServerResponse,
    ExpectedOneSpawnReturnedNone,
}
impl std::error::Error for ClientError {}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ClientError::*;

        match self {
            Deserialization(e) => write!(f, "couldn't parse what server returned: {}", e),
            RawDeserialization(e) => write!(f, "couldn't parse what server returned: {}", e),
            HttpRequest(e) => write!(f, "couldn't communicate with server: {}", e),
            BadRequest(status, e) => write!(f, "server returned Status {}, error: {}", status, e),
            ServerError(status, e) => write!(f, "server returned Status {}, error: {}", status, e),
            UnknownServerResponse => write!(f, "server returned error client doesn't understand."),
            ExpectedOneSpawnReturnedNone => write!(
                f,
                "attempted to spawn a single item, but the server returned no items"
            ),
        }
    }
}
impl From<awc::error::SendRequestError> for ClientError {
    fn from(e: awc::error::SendRequestError) -> ClientError {
        ClientError::HttpRequest(e)
    }
}
impl From<awc::error::JsonPayloadError> for ClientError {
    fn from(e: awc::error::JsonPayloadError) -> ClientError {
        ClientError::Deserialization(e)
    }
}
impl From<awc::error::PayloadError> for ClientError {
    fn from(e: awc::error::PayloadError) -> ClientError {
        ClientError::RawDeserialization(e)
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
