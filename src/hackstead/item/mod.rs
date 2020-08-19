use crate::{config, plant, IdentifiesSteader, ItemId, SteaderId};
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
    pub conf: Conf,
    pub gotchi: Option<Gotchi>,
    pub ownership_log: Vec<LoggedOwner>,
}

#[derive(Deserialize, SerdeDiff, Serialize, Debug, PartialEq, Clone, Copy)]
#[serde(transparent)]
/// An item::Conf points to an item::Config in the CONFIG lazy_static.
pub struct Conf(pub(crate) usize);

impl std::ops::Deref for Conf {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        config::CONFIG
            .items
            .get(self.0)
            .as_ref()
            .expect("invalid item Conf, this is very bad")
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub name: String,

    pub description: String,

    #[serde(default)]
    pub gotchi: Option<gotchi::Config>,

    /// This String needs to get verified into a plant::Conf
    #[serde(default)]
    pub grows_into: Option<String>,

    #[serde(default)]
    pub unlocks_land: Option<LandUnlock>,

    #[serde(default)]
    pub welcome_gift: bool,

    #[serde(default)]
    /// These raw plant effects need to get verified into plant effects
    pub passive_plant_effects: Vec<plant::RawEffectConfig>,

    #[serde(default)]
    /// These raw plant effects need to get verified into plant effects
    pub plant_rub_effects: Vec<plant::RawEffectConfig>,

    #[serde(default)]
    /// This RawEvalput needs to have its item names looked up n verified
    pub hatch_table: Option<config::RawEvalput>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub conf: Conf,
    pub gotchi: Option<gotchi::Config>,
    pub grows_into: Option<plant::Conf>,
    pub unlocks_land: Option<LandUnlock>,
    pub welcome_gift: bool,
    pub passive_plant_effects: Vec<plant::EffectConfig>,
    pub plant_rub_effects: Vec<plant::EffectConfig>,
    pub hatch_table: Option<config::Evalput<Conf>>,
}

impl config::Verify for RawConfig {
    type Verified = Config;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        Ok(Config {
            grows_into: self
                .grows_into
                .as_ref()
                .map(|plant_name| raw.plant_conf(plant_name))
                .transpose()?,
            conf: raw.item_conf(&self.name)?,
            name: self.name,
            description: self.description,
            gotchi: self.gotchi,
            unlocks_land: self.unlocks_land,
            welcome_gift: self.welcome_gift,
            passive_plant_effects: self.passive_plant_effects.verify(raw)?,
            plant_rub_effects: self.plant_rub_effects.verify(raw)?,
            hatch_table: self.hatch_table.verify(raw)?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in the item named {}", self.name))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LandUnlock {
    pub requires_xp: bool,
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

impl Item {
    pub fn from_conf(conf: Conf, owner: impl IdentifiesSteader, acquisition: Acquisition) -> Self {
        let logged_owner_id = owner.steader_id();
        let item_id = ItemId(uuid::Uuid::new_v4());
        Self {
            item_id,
            gotchi: Some(Gotchi::new(conf)).filter(|_| conf.gotchi.is_some()),
            owner_id: logged_owner_id,
            ownership_log: vec![LoggedOwner {
                owner_index: 0,
                logged_owner_id,
                acquisition,
            }],
            conf,
        }
    }

    pub fn nickname(&self) -> &str {
        match &self.gotchi {
            Some(g) => &g.nickname,
            _ => &self.conf.name,
        }
    }
}
