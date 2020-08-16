use crate::id::{NoSuch, NoSuchGotchiOnItem, NoSuchResult};
use crate::{config, ItemId, SteaderId, CONFIG};
use config::{Archetype, ArchetypeHandle, ConfigResult};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;

pub mod gotchi;

pub use gotchi::Gotchi;

#[derive(SerdeDiff, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LoggedOwner {
    pub logged_owner_id: SteaderId,
    pub acquisition: Acquisition,
    pub owner_index: usize,
}

#[derive(SerdeDiff, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    pub fn try_from_usize(i: usize) -> Option<Self> {
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

#[derive(Deserialize, SerdeDiff, Serialize, Debug, PartialEq, Clone)]
pub struct Item {
    pub item_id: ItemId,
    pub owner_id: SteaderId,
    pub archetype_handle: ArchetypeHandle,
    gotchi: Option<Gotchi>,
    pub ownership_log: Vec<LoggedOwner>,
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{ClientError, ClientResult},
        wormhole::{ask, until_ask_id_map, AskedNote, ItemAsk},
        Ask, IdentifiesItem, IdentifiesSteader, Tile,
    };

    impl Item {
        pub async fn redeem_for_tile(&self) -> ClientResult<Tile> {
            let a = Ask::TileSummon {
                tile_redeemable_item_id: self.item_id(),
            };

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::TileSummonResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "TileSummon", e))
        }

        pub async fn hatch(&self) -> ClientResult<Vec<Item>> {
            let a = Ask::Item(ItemAsk::Hatch {
                hatchable_item_id: self.item_id(),
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::ItemHatchResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "ItemHatch", e))
        }

        pub async fn throw_at(&self, to: impl IdentifiesSteader) -> ClientResult<Item> {
            let a = Ask::Item(ItemAsk::Throw {
                receiver_id: to.steader_id(),
                item_ids: vec![self.item_id],
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::ItemThrowResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a.clone(), "ItemThrow", e))?
            .pop()
            .ok_or_else(|| {
                ClientError::bad_ask(a, "ItemThrow", "Asked for one item, got none".to_string())
            })
        }
    }
}

impl std::ops::Deref for Item {
    type Target = Archetype;

    fn deref(&self) -> &Self::Target {
        CONFIG.item(self.archetype_handle).unwrap()
    }
}

impl Item {
    pub fn from_archetype_handle(
        ah: ArchetypeHandle,
        logged_owner_id: SteaderId,
        acquisition: Acquisition,
    ) -> ConfigResult<Self> {
        let a = CONFIG.item(ah)?;
        Self::from_archetype(a, logged_owner_id, acquisition)
    }

    pub fn from_archetype(
        a: &'static Archetype,
        logged_owner_id: SteaderId,
        acquisition: Acquisition,
    ) -> ConfigResult<Self> {
        let item_id = ItemId(uuid::Uuid::new_v4());
        let ah = CONFIG.possession_archetype_to_handle(a)?;
        Ok(Self {
            item_id,
            archetype_handle: ah,
            owner_id: logged_owner_id,
            gotchi: Some(Gotchi::new(ah)).filter(|_| a.gotchi.is_some()),
            ownership_log: vec![LoggedOwner {
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

    pub fn gotchi(&self) -> NoSuchResult<&Gotchi> {
        Ok(self
            .gotchi
            .as_ref()
            .ok_or_else(|| NoSuch::Gotchi(NoSuchGotchiOnItem(self.owner_id, self.item_id)))?)
    }

    pub fn gotchi_mut(&mut self) -> NoSuchResult<&mut Gotchi> {
        let (owner_id, item_id) = (self.owner_id, self.item_id);
        Ok(self
            .gotchi
            .as_mut()
            .ok_or_else(|| NoSuch::Gotchi(NoSuchGotchiOnItem(owner_id, item_id)))?)
    }
}
