use crate::{config, market, CONFIG};
use config::{Archetype, ArchetypeHandle};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod gotchi;
pub mod seed;

pub use gotchi::Gotchi;
pub use seed::Seed;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Owner {
    pub id: String,
    pub acquisition: Acquisition,
}
impl Owner {
    pub fn farmer(id: String) -> Self {
        Self {
            id,
            acquisition: Acquisition::Farmed,
        }
    }
    pub fn crafter(id: String) -> Self {
        Self {
            id,
            acquisition: Acquisition::Crafted,
        }
    }
    pub fn hatcher(id: String) -> Self {
        Self {
            id,
            acquisition: Acquisition::Hatched,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Acquisition {
    Trade,
    Purchase { price: u64 },
    Farmed,
    Crafted,
    Hatched,
}
impl Acquisition {
    pub fn spawned() -> Self {
        Acquisition::Trade
    }
}
impl fmt::Display for Acquisition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Acquisition::Trade => write!(f, "Trade"),
            Acquisition::Farmed => write!(f, "Farmed"),
            Acquisition::Crafted => write!(f, "Crafted"),
            Acquisition::Hatched => write!(f, "Hatched"),
            Acquisition::Purchase { price } => write!(f, "Purchase({}gp)", price),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Item {
    pub seed: Option<Seed>,
    pub gotchi: Option<Gotchi>,
    pub archetype_handle: ArchetypeHandle,
    pub id: uuid::Uuid,
    pub steader: String,
    pub ownership_log: Vec<Owner>,
    pub sale: Option<market::Sale>,
}

impl std::ops::Deref for Item {
    type Target = Archetype;

    fn deref(&self) -> &Self::Target {
        Self::archetype(self.archetype_handle)
    }
}

impl Item {
    pub fn new(ah: ArchetypeHandle, owner: Owner) -> Self {
        let a = Self::archetype(ah);
        Self {
            seed: Some(Seed::new(ah, &owner.id)).filter(|_| a.seed.is_some()),
            gotchi: Some(Gotchi::new(ah, &owner.id)).filter(|_| a.gotchi.is_some()),
            id: uuid::Uuid::new_v4(),
            archetype_handle: ah,
            steader: owner.id.clone(),
            ownership_log: vec![owner],
            sale: None,
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
            .get(ah)
            .expect("invalid archetype handle")
    }
}
