use crate::{config, CONFIG};
use config::{Archetype, ArchetypeHandle, ConfigError, ConfigResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub mod gotchi;

pub use gotchi::Gotchi;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LoggedOwner {
    pub item_id: Uuid,
    pub logged_owner_id: Uuid,
    pub acquisition: Acquisition,
    pub owner_index: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Acquisition {
    Trade,
    Farmed,
    Crafted,
    Hatched,
}
impl Acquisition {
    pub fn spawned() -> Self {
        Acquisition::Trade
    }
    pub fn try_from_i32(i: i32) -> Option<Self> {
        use Acquisition::*;

        Some(match i {
            0 => Trade,
            1 => Farmed,
            2 => Crafted,
            3 => Hatched,
            _ => return None,
        })
    }
}
impl fmt::Display for Acquisition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Acquisition::Trade => write!(f, "Trade"),
            Acquisition::Farmed => write!(f, "Farmed"),
            Acquisition::Crafted => write!(f, "Crafted"),
            Acquisition::Hatched => write!(f, "Hatched"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a new item be created and given to a user.
pub struct ItemTransferRequest {
    /// The steader the items should be transferred to.
    pub receiver_id: crate::UserId,
    /// The steader from whom the items should be transferred.
    pub sender_id: crate::UserId,
    /// The ids of the items to be transferred. Any items referenced which do not belong to the
    /// sender are ignored.
    pub item_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a new item be created and given to a user.
pub struct ItemSpawnRequest {
    /// The steaders the items should be spawned for.
    pub receiver_id: crate::UserId,
    /// A handle to variety of item to be spawned for the user, as specified by the config.
    pub item_archetype_handle: crate::config::ArchetypeHandle,
    /// The number of items the user should receive.
    pub amount: usize,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct ItemBase {
    pub archetype_handle: ArchetypeHandle,
    pub item_id: uuid::Uuid,
    pub owner_id: uuid::Uuid,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Item {
    pub base: ItemBase,
    pub gotchi: Option<Gotchi>,
    pub ownership_log: Vec<LoggedOwner>,
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::client::{ClientResult, IdentifiesItem, CLIENT, SERVER_URL};
    use crate::hackstead::{Tile, TileCreationRequest};

    impl Item {
        pub async fn redeem_for_tile(&self) -> ClientResult<Tile> {
            Ok(CLIENT
                .post(&format!("{}/{}", *SERVER_URL, "tile/new"))
                .json(&TileCreationRequest {
                    tile_redeemable_item_id: self.item_id(),
                })
                .send()
                .await?
                .json()
                .await?)
        }
    }
}

impl std::ops::Deref for Item {
    type Target = Archetype;

    fn deref(&self) -> &Self::Target {
        Self::archetype(self.base.archetype_handle).expect("invalid archetype handle")
    }
}

impl Item {
    pub fn from_archetype_handle(
        ah: ArchetypeHandle,
        logged_owner_id: Uuid,
        acquisition: Acquisition,
    ) -> ConfigResult<Self> {
        let a = Self::archetype(ah)?;
        Self::from_archetype(a, logged_owner_id, acquisition)
    }

    pub fn from_archetype(
        a: &'static Archetype,
        logged_owner_id: Uuid,
        acquisition: Acquisition,
    ) -> ConfigResult<Self> {
        let item_id = uuid::Uuid::new_v4();
        let ah = CONFIG.possession_archetype_to_handle(a)?;
        Ok(Self {
            base: ItemBase {
                item_id,
                archetype_handle: ah,
                owner_id: logged_owner_id,
            },
            gotchi: Some(Gotchi::new(item_id, ah)).filter(|_| a.gotchi.is_some()),
            ownership_log: vec![LoggedOwner {
                item_id,
                owner_index: 0,
                logged_owner_id,
                acquisition,
            }],
        })
    }

    pub fn nickname(&self) -> &str {
        match &self.gotchi {
            Some(g) => &g.nickname,
            _ => &self.name,
        }
    }

    fn archetype(ah: ArchetypeHandle) -> ConfigResult<&'static Archetype> {
        CONFIG
            .possession_archetypes
            .get(ah as usize)
            .ok_or_else(|| ConfigError::UnknownArchetypeHandle(ah))
    }
}
