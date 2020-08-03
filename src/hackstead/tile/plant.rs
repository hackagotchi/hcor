use crate::config;
use config::{ArchetypeHandle, PlantArchetype, CONFIG};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting than an item to be applied to a plant
pub struct PlantRubRequest {
    /// the item to be applied to a plant
    pub rub_item_id: Uuid,
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
    /// Records how many effects have been imparted onto this plant from applied items
    /// over its lifetime (including effects that wore off long ago)
    pub lifetime_effect_count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Plant {
    pub base: PlantBase,
    pub craft: Option<Craft>,
    /// Effects from potions, warp powder, etc. that actively change the behavior of this plant.
    pub effects: Vec<Effect>,
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
                lifetime_effect_count: 0,
            },
            craft: None,
            effects: vec![],
        })
    }

    /// increments the lifetime_effect_count and returns an index suitable for a new effect.
    pub fn next_effect_index(&mut self) -> i32 {
        let e = self.base.lifetime_effect_count;
        self.base.lifetime_effect_count += 1;
        e
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Craft {
    pub tile_id: Uuid,
    pub until_finish: f64,
    #[serde(alias = "makes")]
    pub recipe_archetype_handle: ArchetypeHandle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    /// Records whether this is the first, second, third, etc. effect to be rubbed onto this plant.
    pub rub_index: i32,
    pub tile_id: Uuid,
    pub until_finish: Option<f64>,
    /// The archetype of the item that was consumed to apply this effect.
    pub item_archetype_handle: ArchetypeHandle,
    /// The archetype of the effect within this item that describes this effect.
    pub effect_archetype_handle: ArchetypeHandle,
}
impl std::ops::Deref for Effect {
    type Target = config::PlantRubEffect;

    fn deref(&self) -> &Self::Target {
        &CONFIG
            .plant_rub_effect(self.item_archetype_handle, self.effect_archetype_handle)
            .expect("invalid effect archetype handle")
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{request, ClientResult},
        IdentifiesItem,
    };

    impl Plant {
        pub async fn slaughter(&self) -> ClientResult<Plant> {
            request(
                "plant/slaughter",
                &PlantRemovalRequest {
                    tile_id: self.base.tile_id,
                },
            )
            .await
        }

        pub async fn rub_with(&self, rub: impl IdentifiesItem) -> ClientResult<Vec<Effect>> {
            request(
                "plant/rub",
                &PlantRubRequest {
                    rub_item_id: rub.item_id(),
                    tile_id: self.base.tile_id,
                },
            )
            .await
        }
    }
}
