use crate::{config, item};
use chrono::{DateTime, Utc};
use config::CONFIG;
use item::Item;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod plant;
pub use plant::Plant;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a new hackstead is made for a user
pub struct NewHacksteadRequest {
    /// A slack id to be associated with this user, if any.
    pub slack_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hackstead {
    pub profile: Profile,
    pub land: Vec<Tile>,
    pub inventory: Vec<Item>,
}
impl Hackstead {
    pub fn empty(slack_id: Option<impl ToString>) -> Self {
        Hackstead {
            profile: Profile::new(slack_id.map(|s| s.to_string())),
            land: vec![],
            inventory: vec![],
        }
    }

    pub fn new_user(slack_id: Option<impl ToString>) -> Self {
        let profile = Profile::new(slack_id.map(|s| s.to_string()));
        Hackstead {
            inventory: CONFIG
                .possession_archetypes
                .iter()
                .filter(|a| a.welcome_gift)
                .map(|a| Item::from_archetype(a, profile.steader_id, item::Acquisition::spawned()))
                .collect::<Result<Vec<_>, _>>()
                .expect("fresh possession archetypes somehow invalid"),
            land: vec![Tile::new(profile.steader_id)],
            profile,
        }
    }

    pub fn land_unlock_eligible(&self) -> bool {
        let xp_allows = self.profile.advancements_sum().land;
        let extra = self.profile.extra_land_plot_count;
        let eligible = (xp_allows + extra) as usize;

        self.land.len() < eligible
    }
}
#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::client::{ClientResult, IdentifiesItem, IdentifiesUser, CLIENT, SERVER_URL};

    impl Hackstead {
        pub async fn fetch(iu: impl IdentifiesUser) -> ClientResult<Self> {
            Ok(CLIENT
                .get(&format!("{}/{}", *SERVER_URL, "hackstead/"))
                .json(&iu.user_id())
                .send()
                .await?
                .json()
                .await?)
        }

        pub async fn register() -> ClientResult<Self> {
            Self::register_raw(None).await
        }

        pub async fn register_with_slack(slack: impl ToString) -> ClientResult<Self> {
            Self::register_raw(Some(slack.to_string())).await
        }

        async fn register_raw(slack_id: Option<String>) -> ClientResult<Self> {
            Ok(CLIENT
                .post(&format!("{}/{}", *SERVER_URL, "hackstead/new"))
                .json(&NewHacksteadRequest { slack_id })
                .send()
                .await?
                .json()
                .await?)
        }

        pub async fn slaughter(&self) -> ClientResult<Self> {
            Ok(CLIENT
                .post(&format!("{}/{}", *SERVER_URL, "hackstead/remove"))
                .json(&self.user_id())
                .send()
                .await?
                .json()
                .await?)
        }

        pub fn has_item(&self, ii: impl IdentifiesItem) -> bool {
            let item_id = ii.item_id();
            self.inventory.iter().any(|i| i.base.item_id == item_id)
        }

        pub async fn give_to<'a, I>(
            &self,
            to: impl IdentifiesUser,
            items: &'a [I],
        ) -> ClientResult<Vec<crate::Item>>
        where
            &'a I: IdentifiesItem,
        {
            Ok(CLIENT
                .post(&format!("{}/{}", *SERVER_URL, "item/transfer"))
                .json(&crate::item::ItemTransferRequest {
                    sender_id: self.user_id(),
                    receiver_id: to.user_id(),
                    item_ids: items.iter().map(|i| i.item_id()).collect(),
                })
                .send()
                .await?
                .json()
                .await?)
        }

        pub async fn spawn_items(
            &self,
            item_archetype_handle: crate::config::ArchetypeHandle,
            amount: usize,
        ) -> ClientResult<Vec<crate::Item>> {
            Ok(CLIENT
                .post(&format!("{}/{}", *SERVER_URL, "item/spawn"))
                .json(&crate::item::ItemSpawnRequest {
                    receiver_id: self.user_id(),
                    item_archetype_handle,
                    amount,
                })
                .send()
                .await?
                .json()
                .await?)
        }

        pub async fn unlock_tile_with(&self, item: impl IdentifiesItem) -> ClientResult<Tile> {
            Ok(CLIENT
                .post(&format!("{}/{}", *SERVER_URL, "tile/new"))
                .json(&TileCreationRequest {
                    tile_redeemable_item_id: item.item_id(),
                })
                .send()
                .await?
                .json()
                .await?)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a user's item is consumed in exchange for a new tile of land.
/// The steader to give the land to is inferred to be the owner of the item.
pub struct TileCreationRequest {
    /// id for an item that is capable of being removed in exchange for another tile of land.
    pub tile_redeemable_item_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TileBase {
    pub owner_id: Uuid,
    pub tile_id: Uuid,
    pub acquired: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tile {
    pub plant: Option<Plant>,
    pub base: TileBase,
}
impl Tile {
    pub fn new(owner_id: Uuid) -> Tile {
        Tile {
            plant: None,
            base: TileBase {
                acquired: Utc::now(),
                tile_id: Uuid::new_v4(),
                owner_id,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub steader_id: Uuid,
    pub slack_id: Option<String>,

    pub xp: i32,
    pub extra_land_plot_count: i32,

    /// Indicates when this Hacksteader first joined the elite community.
    pub joined: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
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
            steader_id: Uuid::new_v4(),
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

    pub fn increase_xp(&mut self, amt: i32) -> Option<&config::HacksteadAdvancement> {
        CONFIG
            .profile_archetype
            .advancements
            .increase_xp(&mut self.xp, amt)
    }
}
