use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies a user. They can be identified by their Slack id, or by an uuid that we coined,
/// with a compile time check that they supply one or both.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UserId {
    /// A user known only by an id we coined for them.
    Uuid(Uuid),
    /// A user known only by their slack id.
    Slack(String),
    /// A user known by both their slack id and their uuid.
    Both { uuid: Uuid, slack: String },
}
impl UserId {
    /// Returns an ID that we coined for a user, if available.
    pub fn uuid(&self) -> Option<Uuid> {
        match self {
            UserId::Uuid(uuid) | UserId::Both { uuid, .. } => Some(*uuid),
            _ => None,
        }
    }
    pub fn uuid_or_else(&self, f: impl FnOnce(&str) -> Uuid) -> Uuid {
        match self {
            UserId::Uuid(uuid) | UserId::Both { uuid, .. } => *uuid,
            UserId::Slack(slack) => f(slack),
        }
    }
    /// Returns a slack id for a user, if available.
    pub fn slack(&self) -> Option<&str> {
        match self {
            UserId::Slack(slack) | UserId::Both { slack, .. } => Some(slack),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const USER_1: &'static str = "U1";
    lazy_static::lazy_static! {
        static ref UUID: Uuid = Uuid::new_v4();
    }

    #[test]
    fn slack_id_fetching() {
        let s = UserId::Slack(USER_1.to_string());
        assert_eq!(s.uuid(), None, "slack only id should not have uuid");
        assert_eq!(
            s.slack(),
            Some(USER_1),
            "slack only id doesn't store user properly"
        );
    }

    #[test]
    fn uuid_id_fetching() {
        let e = UserId::Uuid(*UUID);
        assert_eq!(
            e.uuid(),
            Some(*UUID),
            "uuid only id doesn't store uuid properly"
        );
        assert_eq!(e.slack(), None, "uuid only id shouldn't have slack");
    }

    #[test]
    fn both_id_fetching() {
        let both = UserId::Both {
            slack: USER_1.to_string(),
            uuid: *UUID,
        };
        assert_eq!(
            both.slack(),
            Some(USER_1),
            "both id doesn't store slack properly"
        );

        assert_eq!(
            both.uuid(),
            Some(*UUID),
            "both id doesn't store uuid properly"
        );
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
impl IdentifiesTile for &crate::Tile {
    fn tile_id(self) -> Uuid {
        self.base.tile_id
    }
}
impl<T: IdentifiesPlant> IdentifiesTile for T {
    fn tile_id(self) -> Uuid {
        self.tile_id()
    }
}

/// Plants are referred to as tile Ids, but through this interface,
/// Plants can refer to Tiles, but Tiles can't refer to Plants since
/// Tiles may or may not actually have plants on them.
pub trait IdentifiesPlant {
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
impl IdentifiesPlant for &crate::Plant {
    fn tile_id(self) -> Uuid {
        self.base.tile_id
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
