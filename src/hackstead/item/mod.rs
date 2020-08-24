use crate::{config, plant, IdentifiesSteader, ItemId, SteaderId, id::{NoSuch, NoSuchGotchiOnItem, NoSuchResult}};
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
    gotchi: Option<Gotchi>,
    pub ownership_log: Vec<LoggedOwner>,
}

#[derive(Deserialize, SerdeDiff, Serialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
#[serde_diff(opaque)]
/// An item::Conf points to an item::Config in the CONFIG lazy_static.
pub struct Conf(pub(crate) uuid::Uuid);

impl std::ops::Deref for Conf {
    type Target = Config;

    #[cfg(feature = "config_verify")]
    fn deref(&self) -> &Self::Target {
        panic!("no looking up confs with config_verify enabled")
    }

    #[cfg(not(feature = "config_verify"))]
    fn deref(&self) -> &Self::Target {
        config::CONFIG
            .items
            .get(self)
            .as_ref()
            .expect("invalid item Conf, this is very bad")
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub name: String,

    pub description: String,

    pub conf: Conf,

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
    pub passive_plant_effects: Vec<plant::effect::RawConfig>,

    #[serde(default)]
    /// These raw plant effects need to get verified into plant effects
    pub plant_rub_effects: Vec<plant::effect::RawConfig>,

    #[serde(default)]
    /// This RawEvalput needs to have its item names looked up n verified
    pub hatch_table: Option<config::RawEvalput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub conf: Conf,
    pub gotchi: Option<gotchi::Config>,
    pub grows_into: Option<plant::Conf>,
    pub unlocks_land: Option<LandUnlock>,
    pub welcome_gift: bool,
    pub passive_plant_effects: Vec<plant::effect::Config>,
    pub plant_rub_effects: Vec<plant::effect::Config>,
    pub hatch_table: Option<config::Evalput<Conf>>,
}

#[cfg(feature = "config_verify")]
impl config::Verify for RawConfig {
    type Verified = Config;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        Ok(Config {
            grows_into: self
                .grows_into
                .as_ref()
                .map(|plant_name| raw.plant_conf(plant_name))
                .transpose()?,
            conf: self.conf,
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
            gotchi: Some(Gotchi::new(conf, item_id)).filter(|_| conf.gotchi.is_some()),
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
