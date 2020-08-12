use crate::{config::{self, CONFIG}, IdentifiesItem, IdentifiesPlant, IdentifiesTile, IdentifiesSteader, SteaderId, id::{NoSuchPlantOnTile, NoSuchItem, NoSuchTile, NoSuchResult}};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

pub mod tile;
pub use tile::{
    plant::{self, Plant},
    Tile,
};

/// Some items boost the growth of plants; others accelerate their growth or give you more land.
/// This module facilitates handling all of them.
pub mod item;
pub use item::Item;

#[derive(Clone, Debug, SerdeDiff, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "hackstead_message_derive", derive(actix::MessageResponse))]
pub struct Hackstead {
    pub profile: Profile,
    pub land: Vec<Tile>,
    pub inventory: Vec<Item>,
    pub timers: Vec<plant::Timer>,
}
impl Hackstead {
    pub fn empty(slack_id: Option<impl ToString>) -> Self {
        Hackstead {
            profile: Profile::new(slack_id.map(|s| s.to_string())),
            land: vec![],
            inventory: vec![],
            timers: vec![],
        }
    }

    pub fn new_user(slack_id: Option<impl ToString>) -> Self {
        let profile = Profile::new(slack_id.map(|s| s.to_string()));
        Hackstead {
            inventory: CONFIG
                .welcome_gifts()
                .map(|a| Item::from_archetype(a, profile.steader_id, item::Acquisition::spawned()))
                .collect::<Result<Vec<_>, _>>()
                .expect("fresh possession archetypes somehow invalid"),
            land: vec![Tile::new(profile.steader_id)],
            timers: vec![],
            profile,
        }
    }

    /// Returns true if this hackstead has enough xp to redeem another tile of land.
    pub fn land_unlock_eligible(&self) -> bool {
        let xp_allows = self.profile.advancements_sum().land;
        let extra = self.profile.extra_land_plot_count;
        let eligible = (xp_allows + extra) as usize;

        self.land.len() < eligible
    }

    /// Returns an iterator over all tiles which are not occupied by plants.
    pub fn free_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.land.iter().filter(|t| t.plant.is_none())
    }

    /// Returns a tile in this hackstead not occupied by a plant, if any.
    pub fn free_tile(&self) -> Option<Tile> {
        self.free_tiles().next().cloned()
    }

    pub fn plants(&self) -> impl Iterator<Item = &Plant> {
        self.land.iter().filter_map(|t| t.plant.as_ref())
    }

    pub fn plants_mut(&mut self) -> impl Iterator<Item = &mut Plant> {
        self.land.iter_mut().filter_map(|t| t.plant.as_mut())
    }

    pub fn item(&self, i: impl IdentifiesItem) -> NoSuchResult<&Item> {
        let item_id = i.item_id();
        Ok(self.inventory.iter().find(|i| i.item_id == item_id).ok_or_else(|| {
            NoSuchItem(self.steader_id(), item_id)
        })?)
    }

    pub fn take_item(&mut self, i: impl IdentifiesItem) -> NoSuchResult<Item> {
        let (steader_id, item_id) = (self.steader_id(), i.item_id());
        let p = self.inventory.iter().position(|i| i.item_id == item_id).ok_or_else(|| {
            NoSuchItem(steader_id, item_id)
        })?;
        Ok(self.inventory.swap_remove(p))
    }

    pub fn tile(&self, t: impl IdentifiesTile) -> NoSuchResult<&Tile> {
        let (steader_id, tile_id) = (self.steader_id(), t.tile_id());
        Ok(self.land.iter().find(|t| t.tile_id == tile_id).ok_or_else(|| {
            NoSuchTile(steader_id, tile_id)
        })?)
    }

    pub fn tile_mut(&mut self, t: impl IdentifiesTile) -> NoSuchResult<&mut Tile> {
        let (steader_id, tile_id) = (self.steader_id(), t.tile_id());
        Ok(self.land.iter_mut().find(|t| t.tile_id == tile_id).ok_or_else(|| {
            NoSuchTile(steader_id, tile_id)
        })?)
    }

    /// Safely accepts IdentifiesTile instead of IdentifiesPlant because this function will
    /// simply return an error if there's no plant on the tile.
    pub fn plant(&self, t: impl IdentifiesTile) -> NoSuchResult<&Plant> {
        let (steader_id, tile_id) = (self.steader_id(), t.tile_id());
        Ok(self
            .tile(tile_id)?
            .plant
            .as_ref()
            .ok_or_else(|| NoSuchPlantOnTile(steader_id, tile_id))?)
    }

    /// Safely accepts IdentifiesTile instead of IdentifiesPlant because this function will
    /// simply return an error if there's no plant on the tile.
    pub fn plant_mut(&mut self, t: impl IdentifiesTile) -> NoSuchResult<&mut Plant> {
        let (steader_id, tile_id) = (self.steader_id(), t.tile_id());
        Ok(self
            .tile_mut(tile_id)?
            .plant
            .as_mut()
            .ok_or_else(|| NoSuchPlantOnTile(steader_id, tile_id))?)
    }

    /// Safely accepts IdentifiesTile instead of IdentifiesPlant because this function will
    /// simply return an error if there's no plant on the tile.
    pub fn take_plant(&mut self, t: impl IdentifiesTile) -> NoSuchResult<Plant> {
        let (steader_id, tile_id) = (self.steader_id(), t.tile_id());
        let tile = self.tile_mut(tile_id)?;
        Ok(tile.plant.take().ok_or_else(|| NoSuchPlantOnTile(steader_id, tile_id))?)
    }

    pub fn has_item(&self, i: impl IdentifiesItem) -> bool {
        self.item(i).is_ok()
    }

    pub fn has_tile(&self, t: impl IdentifiesTile) -> bool {
        self.tile(t).is_ok()
    }

    pub fn has_plant(&self, p: impl IdentifiesPlant) -> bool {
        self.plant(p).is_ok()
    }
}
#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{request, ClientError, ClientResult},
        wormhole::{self, ask, AskedNote, until_ask_id_map, ItemAsk},
        Ask, IdentifiesSteader, IdentifiesUser, Item, Tile,
    };

    impl Hackstead {
        pub async fn fetch(iu: impl IdentifiesUser) -> ClientResult<Self> {
            request("hackstead/spy", &iu.user_id()).await
        }

        pub async fn register() -> ClientResult<Self> {
            let s = Self::register_raw(None).await?;
            wormhole::connect(&s).await?;
            Ok(s)
        }

        pub async fn register_with_slack(slack: impl ToString) -> ClientResult<Self> {
            Self::register_raw(Some(slack.to_string())).await
        }

        async fn register_raw(slack_id: Option<String>) -> ClientResult<Self> {
            request("hackstead/summon", &NewHacksteadRequest { slack_id }).await
        }

        pub async fn slaughter(&self) -> ClientResult<Self> {
            wormhole::disconnect().await?;
            request("hackstead/slaughter", &self.user_id()).await
        }

        pub async fn throw_items<'a, I>(
            &self,
            to: impl IdentifiesSteader,
            items: &'a [I],
        ) -> ClientResult<Vec<Item>>
        where
            &'a I: IdentifiesItem,
        {
            let a = Ask::Item(ItemAsk::Throw {
                receiver_id: to.steader_id(),
                item_ids: items.iter().map(|i| i.item_id()).collect(),
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::ItemThrowResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "ItemThrow", e))
        }

        pub async fn spawn_items(
            &self,
            item_archetype_handle: crate::config::ArchetypeHandle,
            amount: usize,
        ) -> ClientResult<Vec<Item>> {
            let a = Ask::Item(ItemAsk::Spawn {
                item_archetype_handle,
                amount,
            });
            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::ItemSpawnResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "ItemSpawn", e))
        }

        pub async fn knowledge_snort(
            &self,
            xp: usize,
        ) -> ClientResult<usize> {
            let a = Ask::KnowledgeSnort { xp };
            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::KnowledgeSnortResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "KnowledgeSnort", e))
        }

        pub async fn unlock_tile_with(&self, item: impl IdentifiesItem) -> ClientResult<Tile> {
            let a = Ask::TileSummon {
                tile_redeemable_item_id: item.item_id(),
            };
            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::TileSummonResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "TileSummon", e))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a new hackstead is made for a user
pub struct NewHacksteadRequest {
    /// A slack id to be associated with this user, if any.
    pub slack_id: Option<String>,
}

#[derive(Clone, Debug, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub steader_id: SteaderId,
    pub slack_id: Option<String>,

    pub xp: usize,
    pub extra_land_plot_count: usize,

    /// Indicates when this Hacksteader first joined the elite community.
    #[serde_diff(opaque)]
    pub joined: DateTime<Utc>,
    #[serde_diff(opaque)]
    pub last_active: DateTime<Utc>,
    #[serde_diff(opaque)]
    pub last_farm: DateTime<Utc>,
}
impl std::ops::Deref for Profile {
    type Target = config::ProfileArchetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG.profile_archetype
    }
}
impl Profile {
    pub fn new(slack_id: Option<String>) -> Self {
        Self {
            steader_id: SteaderId(uuid::Uuid::new_v4()),
            slack_id,
            xp: 0,
            extra_land_plot_count: 0,
            joined: Utc::now(),
            last_active: Utc::now(),
            last_farm: Utc::now(),
        }
    }

    // TODO: store xp in advancements so methods like these aren't necessary
    pub fn current_advancement(&self) -> &config::HacksteadAdvancement {
        self.advancements.current(self.xp)
    }

    pub fn next_advancement(&self) -> Option<&config::HacksteadAdvancement> {
        self.advancements.next(self.xp)
    }

    pub fn advancements_sum(&self) -> config::HacksteadAdvancementSum {
        self.advancements.sum(self.xp, std::iter::empty())
    }

    pub fn increase_xp(&mut self, amt: usize) -> Option<&config::HacksteadAdvancement> {
        CONFIG
            .profile_archetype
            .advancements
            .increase_xp(&mut self.xp, amt)
    }
}
