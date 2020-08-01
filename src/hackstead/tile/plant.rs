use crate::config;
use config::{ArchetypeHandle, PlantArchetype, CONFIG};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting than an item to be applied to a plant
pub struct PlantApplicationRequest {
    /// the item to be applied to a plant
    pub applicable_item_id: Uuid,
    /// the tile that the plant to apply this to rests on
    pub tile_id: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a plant begin crafting something.
pub struct PlantCraftRequest {
    /// The tile that the plant that should craft sits on
    pub tile_id: Uuid,
    /// The index of the recipe in the list of this plant's recipes
    pub recipe_index: ArchetypeHandle,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a seed be consumed to create a new plant on an empty tile.
pub struct PlantCreationRequest {
    /// The (unoccupied) tile that the new plant should sit on
    pub tile_id: Uuid,
    /// The seed to consume to create the new plant from
    pub seed_item_id: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting than a plant be removed from its tile
pub struct PlantRemovalRequest {
    /// The tile that the plant the user wants to remove sits on
    pub tile_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlantBase {
    pub tile_id: Uuid,
    pub xp: i32,
    pub until_yield: f64,
    pub nickname: String,
    pub archetype_handle: ArchetypeHandle,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Plant {
    pub base: PlantBase,
    pub craft: Option<Craft>,
    /// Effects from potions, warp powder, etc. that actively change the behavior of this plant.
    pub effects: Vec<Effect>,
    /// This field isn't saved to the database, and is just used when `plant.increase_xp()` is called.
    #[serde(skip)]
    pub queued_xp_bonus: i32,
}
impl std::ops::Deref for Plant {
    type Target = PlantArchetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG
            .plant_archetypes
            .get(self.base.archetype_handle as usize)
            .expect("invalid archetype handle")
    }
}
impl Plant {
    pub fn from_seed(tile_id: Uuid, seed: &config::SeedArchetype) -> crate::ConfigResult<Self> {
        let archetype_handle = CONFIG.find_plant_handle(&seed.grows_into)?;
        let arch = CONFIG
            .plant_archetypes
            .get(archetype_handle as usize)
            .unwrap();

        Ok(Self {
            base: PlantBase {
                tile_id,
                xp: 0,
                until_yield: arch.base_yield_duration.unwrap_or(0.0),
                nickname: arch.name.clone(),
                archetype_handle,
            },
            craft: None,
            effects: vec![],
            queued_xp_bonus: 0,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Craft {
    pub tile_id: Uuid,
    pub until_finish: f64,
    #[serde(alias = "makes")]
    pub recipe_archetype_handle: ArchetypeHandle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    pub tile_id: Uuid,
    pub until_finish: Option<f64>,
    /// The archetype of the item that was consumed to apply this effect.
    pub item_archetype_handle: ArchetypeHandle,
    /// The archetype of the effect within this item that describes this effect.
    pub effect_archetype_handle: ArchetypeHandle,
}
impl std::ops::Deref for Effect {
    type Target = config::Archetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG
            .possession_archetypes
            .get(self.item_archetype_handle as usize)
            .expect("invalid archetype handle")
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::client::{client, ClientResult, IdentifiesItem, SERVER_URL};

    impl Plant {
        pub async fn slaughter(&self) -> ClientResult<Plant> {
            Ok(client()
                .post(&format!("{}/{}", *SERVER_URL, "plant/remove"))
                .send_json(&PlantRemovalRequest {
                    tile_id: self.base.tile_id,
                })
                .await?
                .json()
                .await?)
        }

        pub async fn apply_item(
            &self,
            applicable: impl IdentifiesItem,
        ) -> ClientResult<Vec<Effect>> {
            Ok(client()
                .post(&format!("{}/{}", *SERVER_URL, "plant/apply"))
                .send_json(&PlantApplicationRequest {
                    applicable_item_id: applicable.item_id(),
                    tile_id: self.base.tile_id,
                })
                .await?
                .json()
                .await?)
        }
    }
}
