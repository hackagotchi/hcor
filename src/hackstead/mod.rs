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

    pub fn land_unlock_eligible(&self) -> bool {
        let xp_allows = self.profile.advancements_sum().land;
        let extra = self.profile.extra_land_plot_count;
        let eligible = (dbg!(xp_allows) + dbg!(extra)) as usize;

        self.land.len() < eligible
    }

    pub fn new_user(slack_id: Option<impl ToString>) -> Self {
        let profile = Profile::new(slack_id.map(|s| s.to_string()));
        Hackstead {
            inventory: CONFIG
                .possession_archetypes
                .iter()
                .filter(|a| a.welcome_gift)
                .map(|a| Item::from_archetype(
                    a,
                    profile.steader_id,
                    item::Acquisition::spawned()
                ))
                .collect(),
            land: vec![Tile::new(profile.steader_id)],
            profile,
        }
    }

    #[cfg(feature = "client")]
    pub async fn fetch(uc: crate::UserId) -> Result<Self, crate::BackendError> {
        Ok(reqwest::Client::new()
            .get("http://127.0.0.1:8000/hackstead/")
            .json(&uc)
            .send()
            .await?
            .json()
            .await?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a user's item is consumed in exchange for a new tile of land.
pub struct TileCreationRequest {
    /// id for an item that is capable of being removed in exchange for another tile of land.
    pub tile_consumable_item_id: Uuid,
    /// contact info for the steader who owns this item
    pub steader: crate::UserId,
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
