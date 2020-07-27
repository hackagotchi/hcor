use crate::{config, item};
use chrono::{DateTime, Utc};
use config::CONFIG;
use item::Item;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod plant;
pub use plant::Plant;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hackstead {
    pub profile: Profile,
    pub land: Vec<Tile>,
    pub inventory: Vec<Item>,
}
impl Hackstead {
    pub fn new<T: ToString>(slack_id: Option<T>) -> Self {
        Hackstead {
            profile: Profile::new(slack_id.map(|s| s.to_string())),
            land: vec![],
            inventory: vec![],
        }
    }
    #[cfg(feature = "client")]
    pub async fn fetch(uc: crate::UserId) -> Result<Self, crate::BackendError> {
        let client = reqwest::Client::new();
        Ok(client
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
pub struct Tile {
    pub acquired: DateTime<Utc>,
    pub plant: Option<Plant>,
    pub id: Uuid,
    pub steader: String,
}
impl Tile {
    pub fn new(steader: String) -> Tile {
        Tile {
            acquired: Utc::now(),
            plant: None,
            id: Uuid::new_v4(),
            steader,
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
