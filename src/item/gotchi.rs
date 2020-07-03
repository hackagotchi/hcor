use crate::{config, CONFIG};
use config::ArchetypeHandle;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct GotchiHarvestOwner {
    pub id: String,
    #[serde(with = "bson::compat::u2f")]
    pub harvested: u64,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Gotchi {
    archetype_handle: ArchetypeHandle,
    pub nickname: String,
    pub harvest_log: Vec<GotchiHarvestOwner>,
}
impl std::ops::Deref for Gotchi {
    type Target = config::GotchiArchetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG
            .possession_archetypes
            .get(self.archetype_handle)
            .expect("invalid archetype handle")
            .gotchi
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "gotchi has non-gotchi archetype handle {}",
                    self.archetype_handle
                )
            })
    }
}
impl Gotchi {
    pub fn new(archetype_handle: ArchetypeHandle, owner_id: &str) -> Self {
        Self {
            archetype_handle,
            nickname: CONFIG.possession_archetypes[archetype_handle].name.clone(),
            harvest_log: vec![GotchiHarvestOwner {
                id: owner_id.to_string(),
                harvested: 0,
            }],
        }
    }
}
