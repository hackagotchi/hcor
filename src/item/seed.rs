use crate::{config, CONFIG};
use config::ArchetypeHandle;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Seed {
    #[serde(with = "bson::compat::u2f")]
    pub archetype_handle: ArchetypeHandle,
    pub pedigree: Vec<SeedGrower>,
}
impl std::ops::Deref for Seed {
    type Target = config::SeedArchetype;

    fn deref(&self) -> &Self::Target {
        CONFIG
            .possession_archetypes
            .get(self.archetype_handle as usize)
            .expect("invalid archetype handle")
            .seed
            .as_ref()
            .unwrap_or_else(|| {
                panic!("archetype kind corresponds to archetype of a different type")
            })
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
