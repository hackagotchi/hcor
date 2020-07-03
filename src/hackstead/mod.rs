use crate::{config, item};
use chrono::{NaiveDateTime, Utc};
use config::CONFIG;
use item::Item;
use serde::{Deserialize, Serialize};

mod plant;
pub use plant::Plant;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hackstead {
    pub id: String,
    pub profile: Profile,
    pub land: Vec<Tile>,
    pub inventory: Vec<Item>,
}
impl Hackstead {
    pub fn new<T: ToString>(owner_id: T) -> Self {
        Hackstead {
            id: owner_id.to_string(),
            profile: Profile::new(owner_id.to_string()),
            land: vec![],
            inventory: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tile {
    pub acquired: NaiveDateTime,
    pub plant: Option<Plant>,
    pub id: uuid::Uuid,
    pub steader: String,
}
impl Tile {
    pub fn new(steader: String) -> Tile {
        Tile {
            acquired: Utc::now().naive_utc(),
            plant: None,
            id: uuid::Uuid::new_v4(),
            steader,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    /// Indicates when this Hacksteader first joined the elite community.
    pub joined: NaiveDateTime,
    pub last_active: NaiveDateTime,
    pub last_farm: NaiveDateTime,
    /// This is not an uuid::Uuid because it's actually the steader id of the person who owns this Profile
    pub id: String,
    #[serde(with = "bson::compat::u2f")]
    pub xp: u64,
}
impl std::ops::Deref for Profile {
    type Target = config::ProfileArchetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG.profile_archetype
    }
}
impl Profile {
    pub fn new(owner_id: String) -> Self {
        Self {
            joined: Utc::now().naive_utc(),
            last_active: Utc::now().naive_utc(),
            last_farm: Utc::now().naive_utc(),
            xp: 0,
            id: owner_id,
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

    pub fn increase_xp(&mut self, amt: u64) -> Option<&config::HacksteadAdvancement> {
        CONFIG
            .profile_archetype
            .advancements
            .increase_xp(&mut self.xp, amt)
    }
}
