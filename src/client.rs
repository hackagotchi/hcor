use crate::UserId;
use std::fmt;
use uuid::Uuid;

lazy_static::lazy_static! {
    pub static ref CLIENT: reqwest::Client = reqwest::Client::new();
    pub static ref SERVER_URL: &'static str = "http://127.0.0.1:8000";
}

pub type ClientResult<T> = Result<T, ClientError>;

/// Something went wrong while trying to fetch some information from a Hackagotchi backend.
#[derive(Debug)]
pub enum ClientError {
    Deserialization(serde_json::Error),
    HttpRequest(reqwest::Error),
    ExpectedOneSpawnReturnedNone,
}
impl std::error::Error for ClientError {}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ClientError::*;

        match self {
            Deserialization(e) => write!(f, "couldn't parse what server returned: {}", e),
            HttpRequest(e) => write!(f, "server returned error: {}", e),
            ExpectedOneSpawnReturnedNone => write!(
                f,
                "attempted to spawn a single item, but the server returned no items"
            ),
        }
    }
}
impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> ClientError {
        ClientError::Deserialization(e)
    }
}
impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> ClientError {
        ClientError::HttpRequest(e)
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
