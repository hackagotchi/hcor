use super::{Possessable, PossessionKind};
use crate::{config, CONFIG};
use config::{ArchetypeHandle, ArchetypeKind};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Seed {
    pub archetype_handle: ArchetypeHandle,
    pub pedigree: Vec<SeedGrower>,
}
impl Possessable for Seed {
    fn from_possession_kind(pk: PossessionKind) -> Option<Self> {
        pk.as_seed()
    }
    fn into_possession_kind(self) -> PossessionKind {
        PossessionKind::Seed(self)
    }
}
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct SeedGrower {
    pub id: String,
    #[serde(with = "bson::compat::u2f")]
    pub generations: u64,
}
impl SeedGrower {
    pub fn new(id: String, generations: u64) -> Self {
        SeedGrower { id, generations }
    }
}

impl std::ops::Deref for Seed {
    type Target = config::SeedArchetype;

    fn deref(&self) -> &Self::Target {
        match CONFIG
            .possession_archetypes
            .get(self.archetype_handle)
            .expect("invalid archetype handle")
            .kind
        {
            ArchetypeKind::Seed(ref s) => s,
            _ => panic!("archetype kind corresponds to archetype of a different type"),
        }
    }
}
impl Seed {
    pub fn new(archetype_handle: ArchetypeHandle, owner_id: &str) -> Self {
        Self {
            archetype_handle,
            pedigree: vec![SeedGrower {
                id: owner_id.to_string(),
                generations: 0,
            }],
        }
    }
}
