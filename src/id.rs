use crate::plant::RubEffectId;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum NoSuch {
    Plant(NoSuchPlantOnTile),
    Item(NoSuchItem),
    Tile(NoSuchTile),
<<<<<<< HEAD
    Effect(NoSuchRubEffectOnPlant),
=======
    Effect(NoSuchEffectOnPlant),
    Gotchi(NoSuchGotchiOnItem),
>>>>>>> f24160a... feat: Allow renaming gotchi and plants
}
pub type NoSuchResult<T> = Result<T, NoSuch>;
impl std::error::Error for NoSuch {}
impl fmt::Display for NoSuch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NoSuch::Plant(nspot) => write!(f, "{}", nspot),
            NoSuch::Item(nsi) => write!(f, "{}", nsi),
            NoSuch::Tile(nst) => write!(f, "{}", nst),
            NoSuch::Effect(nseop) => write!(f, "{}", nseop),
            NoSuch::Gotchi(nsgoi) => write!(f, "{}", nsgoi),
        }
    }
}
impl From<NoSuchItem> for NoSuch {
    fn from(nsi: NoSuchItem) -> Self {
        NoSuch::Item(nsi)
    }
}
impl From<NoSuchTile> for NoSuch {
    fn from(nst: NoSuchTile) -> Self {
        NoSuch::Tile(nst)
    }
}
impl From<NoSuchPlantOnTile> for NoSuch {
    fn from(nspot: NoSuchPlantOnTile) -> Self {
        NoSuch::Plant(nspot)
    }
}
impl From<NoSuchRubEffectOnPlant> for NoSuch {
    fn from(nseon: NoSuchRubEffectOnPlant) -> Self {
        NoSuch::Effect(nseon)
    }
}
impl From<NoSuchGotchiOnItem> for NoSuch {
    fn from(nsgoi: NoSuchGotchiOnItem) -> Self {
        NoSuch::Gotchi(nsgoi)
    }
}

#[derive(Debug)]
pub struct NoSuchItem(pub SteaderId, pub ItemId);
impl std::error::Error for NoSuchItem {}
impl fmt::Display for NoSuchItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self(sr, i) = self;
        write!(f, "steader {} has no such item {}", sr, i)
    }
}

#[derive(Debug)]
pub struct NoSuchTile(pub SteaderId, pub TileId);
impl std::error::Error for NoSuchTile {}
impl fmt::Display for NoSuchTile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self(sr, t) = self;
        write!(f, "steader {} has no such tile {}", sr, t)
    }
}

#[derive(Debug)]
pub struct NoSuchPlantOnTile(pub SteaderId, pub TileId);
impl std::error::Error for NoSuchPlantOnTile {}
impl fmt::Display for NoSuchPlantOnTile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self(sr, t) = self;
        write!(
            f,
            "steader {} has tile {}, but it's not occupied by a plant.",
            sr, t
        )
    }
}

#[derive(Debug)]
pub struct NoSuchRubEffectOnPlant(pub SteaderId, pub TileId, pub RubEffectId);
impl std::error::Error for NoSuchRubEffectOnPlant {}
impl fmt::Display for NoSuchRubEffectOnPlant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self(sr, t, e) = self;
        write!(
            f,
            "steader {} has a tile {} with a plant on it, \
               but that plant is missing the effect {}.",
            sr, t, e
        )
    }
}

#[derive(Debug)]
pub struct NoSuchGotchiOnItem(pub SteaderId, pub ItemId);
impl std::error::Error for NoSuchGotchiOnItem {}
impl fmt::Display for NoSuchGotchiOnItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self(sr, i) = self;
        write!(f, "steader {} has an item {} with no gotchi", sr, i)
    }
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
#[serde_diff(opaque)]
pub struct TileId(pub Uuid);

impl fmt::Display for TileId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
#[serde_diff(opaque)]
pub struct SteaderId(pub Uuid);

impl fmt::Display for SteaderId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
#[serde_diff(opaque)]
pub struct ItemId(pub Uuid);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifies a user. They can be identified by their Slack id, or by an uuid that we coined,
/// with a compile time check that they supply one or both.
#[derive(SerdeDiff, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UserId {
    /// A user known only by an id we coined for them.
    Uuid(SteaderId),
    /// A user known only by their slack id.
    Slack(String),
    /// A user known by both their slack id and their uuid.
    Both { uuid: SteaderId, slack: String },
}
impl UserId {
    /// Returns an ID that we coined for a user, if available.
    pub fn uuid(&self) -> Option<SteaderId> {
        match self {
            UserId::Uuid(uuid) | UserId::Both { uuid, .. } => Some(*uuid),
            _ => None,
        }
    }
    pub fn uuid_or_else(&self, f: impl FnOnce(&str) -> SteaderId) -> SteaderId {
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
        static ref UUID: SteaderId = SteaderId(Uuid::new_v4());
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
    fn steader_id(self) -> SteaderId;
}
impl IdentifiesSteader for SteaderId {
    fn steader_id(self) -> SteaderId {
        self
    }
}
impl IdentifiesSteader for &SteaderId {
    fn steader_id(self) -> SteaderId {
        *self
    }
}
impl IdentifiesSteader for &crate::Hackstead {
    fn steader_id(self) -> SteaderId {
        self.profile.steader_id
    }
}
impl IdentifiesSteader for &mut crate::Hackstead {
    fn steader_id(self) -> SteaderId {
        self.profile.steader_id
    }
}
impl IdentifiesSteader for &crate::hackstead::Profile {
    fn steader_id(self) -> SteaderId {
        self.steader_id
    }
}
impl IdentifiesSteader for &mut crate::hackstead::Profile {
    fn steader_id(self) -> SteaderId {
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
    fn tile_id(self) -> TileId;
}
impl IdentifiesTile for &crate::Tile {
    fn tile_id(self) -> TileId {
        self.tile_id
    }
}
impl IdentifiesTile for TileId {
    fn tile_id(self) -> TileId {
        self
    }
}
impl IdentifiesTile for &TileId {
    fn tile_id(self) -> TileId {
        *self
    }
}
impl<T: IdentifiesPlant> IdentifiesTile for T {
    fn tile_id(self) -> TileId {
        self.tile_id()
    }
}

/// Plants are referred to as tile Ids, but through this interface,
/// Plants can refer to Tiles, but Tiles can't refer to Plants since
/// Tiles may or may not actually have plants on them.
pub trait IdentifiesPlant {
    fn tile_id(self) -> TileId;
}
impl IdentifiesPlant for &crate::Plant {
    fn tile_id(self) -> TileId {
        self.tile_id
    }
}

pub trait IdentifiesItem {
    fn item_id(self) -> ItemId;
}
impl IdentifiesItem for ItemId {
    fn item_id(self) -> ItemId {
        self
    }
}
impl IdentifiesItem for &ItemId {
    fn item_id(self) -> ItemId {
        *self
    }
}
impl IdentifiesItem for &crate::Item {
    fn item_id(self) -> ItemId {
        self.item_id
    }
}
impl IdentifiesItem for &mut crate::Item {
    fn item_id(self) -> ItemId {
        self.item_id
    }
}
