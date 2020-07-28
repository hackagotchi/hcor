use crate::{config, CONFIG};
use config::{Archetype, ArchetypeHandle};
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

impl std::ops::Deref for Item {
    type Target = Archetype;

    fn deref(&self) -> &Self::Target {
        Self::archetype(self.base.archetype_handle)
    }
}

impl Item {
    pub fn from_archetype_handle(
        ah: ArchetypeHandle,
        logged_owner_id: Uuid,
        acquisition: Acquisition,
    ) -> Self {
        let a = Self::archetype(ah);
        Self::from_archetype(a, logged_owner_id, acquisition)
    }

    pub fn from_archetype(
        a: &'static Archetype,
        logged_owner_id: Uuid,
        acquisition: Acquisition,
    ) -> Self {
        let item_id = uuid::Uuid::new_v4();
        let ah = CONFIG
            .possession_archetype_to_handle(a)
            .expect("invalid archetype");
        Self {
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
        }
    }

    pub fn nickname(&self) -> &str {
        match &self.gotchi {
            Some(g) => &g.nickname,
            _ => &self.name,
        }
    }

    fn archetype(ah: ArchetypeHandle) -> &'static Archetype {
        CONFIG
            .possession_archetypes
            .get(ah as usize)
            .expect("invalid archetype handle")
    }
}
