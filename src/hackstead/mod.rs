use crate::config::{self, CONFIG};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod tile;
pub use tile::{
    plant::{self, Plant},
    Tile,
};

/// Some items boost the growth of plants; others accelerate their growth or give you more land.
/// This module facilitates handling all of them.
pub mod item;
pub use item::Item;

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

    /// Returns true if this hackstead has enough xp to redeem another tile of land.
    pub fn land_unlock_eligible(&self) -> bool {
        let xp_allows = self.profile.advancements_sum().land;
        let extra = self.profile.extra_land_plot_count;
        let eligible = (xp_allows + extra) as usize;

        self.land.len() < eligible
    }

    /// Returns an iterator over all tiles which are not occupied by plants.
    pub fn open_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.land.iter().filter(|t| t.plant.is_none())
    }
}
#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::client::{
        extract_error_or_parse, ClientResult, IdentifiesItem, IdentifiesUser, CLIENT, SERVER_URL,
    };

    impl Hackstead {
        pub async fn fetch(iu: impl IdentifiesUser) -> ClientResult<Self> {
            extract_error_or_parse(
                CLIENT
                    .get(&format!("{}/{}", *SERVER_URL, "hackstead/"))
                    .json(&iu.user_id())
                    .send()
                    .await?,
            )
            .await
        }

        pub async fn register() -> ClientResult<Self> {
            Self::register_raw(None).await
        }

        pub async fn register_with_slack(slack: impl ToString) -> ClientResult<Self> {
            Self::register_raw(Some(slack.to_string())).await
        }

        async fn register_raw(slack_id: Option<String>) -> ClientResult<Self> {
            extract_error_or_parse(
                CLIENT
                    .post(&format!("{}/{}", *SERVER_URL, "hackstead/new"))
                    .json(&NewHacksteadRequest { slack_id })
                    .send()
                    .await?,
            )
            .await
        }

        pub async fn slaughter(&self) -> ClientResult<Self> {
            extract_error_or_parse(
                CLIENT
                    .post(&format!("{}/{}", *SERVER_URL, "hackstead/remove"))
                    .json(&self.user_id())
                    .send()
                    .await?,
            )
            .await
        }

        pub fn has_item(&self, ii: impl IdentifiesItem) -> bool {
            let item_id = ii.item_id();
            self.inventory.iter().any(|i| i.base.item_id == item_id)
        }

        pub async fn give_items<'a, I>(
            &self,
            to: impl IdentifiesUser,
            items: &'a [I],
        ) -> ClientResult<Vec<crate::Item>>
        where
            &'a I: IdentifiesItem,
        {
            extract_error_or_parse(
                CLIENT
                    .post(&format!("{}/{}", *SERVER_URL, "item/transfer"))
                    .json(&crate::item::ItemTransferRequest {
                        sender_id: self.user_id(),
                        receiver_id: to.user_id(),
                        item_ids: items.iter().map(|i| i.item_id()).collect(),
                    })
                    .send()
                    .await?,
            )
            .await
        }

        pub async fn spawn_items(
            &self,
            item_archetype_handle: crate::config::ArchetypeHandle,
            amount: usize,
        ) -> ClientResult<Vec<crate::Item>> {
            extract_error_or_parse(
                CLIENT
                    .post(&format!("{}/{}", *SERVER_URL, "item/spawn"))
                    .json(&crate::item::ItemSpawnRequest {
                        receiver_id: self.user_id(),
                        item_archetype_handle,
                        amount,
                    })
                    .send()
                    .await?,
            )
            .await
        }

        pub async fn unlock_tile_with(&self, item: impl IdentifiesItem) -> ClientResult<Tile> {
            extract_error_or_parse(
                CLIENT
                    .post(&format!("{}/{}", *SERVER_URL, "tile/new"))
                    .json(&tile::TileCreationRequest {
                        tile_redeemable_item_id: item.item_id(),
                    })
                    .send()
                    .await?,
            )
            .await
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
