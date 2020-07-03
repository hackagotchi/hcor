#![feature(try_trait)]
//#![warn(missing_docs)]

use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

pub mod errors;

pub mod category;
pub mod config;

pub mod market;
pub mod possess;
pub mod hackstead;
pub use hackstead::Hackstead;

pub mod frontend {
    pub fn emojify<S: ToString>(txt: S) -> String {
        format!(":{}:", txt.to_string().replace(" ", "_"))
    }
}

pub use category::{Category, CategoryError};
pub use config::CONFIG;
pub use possess::{Possessed, Possession};

pub const TABLE_NAME: &'static str = "hackagotchi";
pub type Item = HashMap<String, AttributeValue>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UserContact {
    Email(String),
    Slack(String),
    Both { email: String, slack: String },
}
impl UserContact {
    pub fn email(&self) -> Option<&str> {
        Some(match self {
            UserContact::Email(s) => s,
            UserContact::Both { email, .. } => email,
            _ => return None,
        })
    }
    pub fn slack(&self) -> Option<&str> {
        Some(match self {
            UserContact::Slack(s) => s,
            UserContact::Both { slack, .. } => slack,
            _ => return None,
        })
    }
}


#[cfg(test)]
mod test {
    use super::*;
    const USER_1: &'static str = "U1";
    const USER_2: &'static str = "U2";
    const USER_3: &'static str = "U3";

    #[test]
    fn slack_contact_fetching() {
        let s = UserContact::Slack(USER_1.to_string());
        assert_eq!(s.email(), None, "slack only contact should not have email");
        assert_eq!(
            s.slack(),
            Some(USER_1),
            "slack only contact doesn't store user properly"
        );
    }

    #[test]
    fn email_contact_fetching() {
        let e = UserContact::Email(USER_2.to_string());
        assert_eq!(
            e.email(),
            Some(USER_2),
            "email only contact doesn't store email properly"
        );
        assert_eq!(e.slack(), None, "email only contact shouldn't have slack");
    }

    #[test]
    fn both_contact_fetching() {
        let both = UserContact::Both {
            slack: USER_1.to_string(),
            email: USER_3.to_string(),
        };
        assert_eq!(
            both.slack(),
            Some(USER_1),
            "both contact doesn't store slack properly"
        );

        assert_eq!(
            both.email(),
            Some(USER_3),
            "both contact doesn't store email properly"
        );
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum AttributeParseError {
    IntFieldParse(&'static str, std::num::ParseIntError),
    FloatFieldParse(&'static str, std::num::ParseFloatError),
    TimeFieldParse(&'static str, humantime::TimestampError),
    IdFieldParse(&'static str, uuid::Error),
    CategoryParse(CategoryError),
    MissingField(&'static str),
    WronglyTypedField(&'static str),
    WrongType,
    Unknown,
    Custom(&'static str),
}
impl Into<String> for AttributeParseError {
    fn into(self) -> String {
        format!("{}", self)
    }
}
impl From<CategoryError> for AttributeParseError {
    fn from(o: CategoryError) -> Self {
        AttributeParseError::CategoryParse(o)
    }
}
impl From<std::option::NoneError> for AttributeParseError {
    fn from(_: std::option::NoneError) -> Self {
        AttributeParseError::Unknown
    }
}

impl fmt::Display for AttributeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AttributeParseError::*;
        match self {
            IntFieldParse(field, e) => write!(f, "error parsing integer field {:?}: {}", field, e),
            FloatFieldParse(field, e) => write!(f, "error parsing float field {:?}: {}", field, e),
            TimeFieldParse(field, e) => {
                write!(f, "error parsing timestamp field {:?}: {}", field, e)
            }
            IdFieldParse(field, e) => write!(f, "error parsing id field {:?}: {}", field, e),
            MissingField(field) => write!(f, "missing field {:?}", field),
            CategoryParse(e) => write!(f, "failed parsing category {}", e),
            WronglyTypedField(field) => write!(f, "wrongly typed field {:?}", field),
            WrongType => write!(f, "wrong AttributeValue type"),
            Unknown => write!(f, "unknown parsing error"),
            Custom(e) => write!(f, "{}", e),
        }
    }
}

/// A model for all keys that use uuid:Uuids internally,
/// essentially all those except Profile keys.
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
pub struct Key {
    pub category: Category,
    pub id: uuid::Uuid,
}
impl Key {
    #[allow(dead_code)]
    pub fn gotchi(id: uuid::Uuid) -> Self {
        Self {
            category: Category::Gotchi,
            id,
        }
    }
    pub fn misc(id: uuid::Uuid) -> Self {
        Self {
            category: Category::Misc,
            id,
        }
    }
    pub fn tile(id: uuid::Uuid) -> Self {
        Self {
            category: Category::Land,
            id,
        }
    }

    pub fn into_item(self) -> Item {
        [
            ("cat".to_string(), self.category.into_av()),
            (
                "id".to_string(),
                AttributeValue {
                    s: Some(self.id.to_string()),
                    ..Default::default()
                },
            ),
        ]
        .iter()
        .cloned()
        .collect()
    }

    pub fn from_item(i: &Item) -> Result<Self, AttributeParseError> {
        use AttributeParseError::*;

        Ok(Self {
            category: Category::from_av(i.get("cat").ok_or(MissingField("cat"))?)?,
            id: uuid::Uuid::parse_str(
                i.get("id")
                    .ok_or(MissingField("id"))?
                    .s
                    .as_ref()
                    .ok_or(WronglyTypedField("id"))?,
            )
            .map_err(|e| IdFieldParse("id", e))?,
        })
    }

    pub async fn fetch_db(self, db: &DynamoDbClient) -> Result<Possession, String> {
        match db
            .get_item(rusoto_dynamodb::GetItemInput {
                key: self.clone().into_item(),
                table_name: TABLE_NAME.to_string(),
                ..Default::default()
            })
            .await
        {
            Ok(o) => {
                Possession::from_item(&o.item.ok_or_else(|| format!("key[{:?}] not in db", self))?)
                    .map_err(|e| format!("couldn't parse item: {}", e))
            }
            Err(e) => Err(format!("Couldn't read key[{:?}] from db: {}", self, e)),
        }
    }
}
