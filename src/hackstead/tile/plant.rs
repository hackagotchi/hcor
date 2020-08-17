use crate::{ config,
    id::{NoSuchEffectOnPlant, NoSuchResult},
    IdentifiesSteader, IdentifiesTile, SteaderId, TileId,
};
use config::{ArchetypeHandle, PlantArchetype, CONFIG};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;

#[derive(Clone, Debug, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Plant {
    pub owner_id: SteaderId,
    pub tile_id: TileId,
    pub xp: usize,
    pub nickname: String,
    pub archetype_handle: ArchetypeHandle,
    /// Records how many items have been applied to this plant
    /// over its lifetime (including effects that wore off long ago)
    pub lifetime_rubs: usize,
    pub craft: Option<Craft>,
    /// Effects from potions, warp powder, etc. that actively change the behavior of this plant.
    pub effects: Vec<Effect>,
}
impl std::ops::Deref for Plant {
    type Target = PlantArchetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG
            .plant_archetypes
            .get(self.archetype_handle as usize)
            .expect("invalid archetype handle")
    }
}
impl Plant {
    pub fn from_seed(
        iu: impl IdentifiesSteader,
        it: impl IdentifiesTile,
        seed: &config::SeedArchetype,
    ) -> crate::ConfigResult<Self> {
        let archetype_handle = CONFIG.find_plant_handle(&seed.grows_into)?;
        let arch = CONFIG
            .plant_archetypes
            .get(archetype_handle as usize)
            .unwrap();

        Ok(Self {
            owner_id: iu.steader_id(),
            tile_id: it.tile_id(),
            xp: 0,
            nickname: arch.name.clone(),
            archetype_handle,
            lifetime_rubs: 0,
            craft: None,
            effects: vec![],
        })
    }

    pub fn effect(&self, effect_id: EffectId) -> NoSuchResult<&Effect> {
        let &Self {
            owner_id,
            tile_id,
            ref effects,
            ..
        } = self;
        Ok(effects
            .iter()
            .find(|e| e.effect_id == effect_id)
            .ok_or_else(|| NoSuchEffectOnPlant(owner_id, tile_id, effect_id))?)
    }

    pub fn effect_mut(&mut self, effect_id: EffectId) -> NoSuchResult<&mut Effect> {
        let &mut Self {
            owner_id,
            tile_id,
            ref mut effects,
            ..
        } = self;
        Ok(effects
            .iter_mut()
            .find(|e| e.effect_id == effect_id)
            .ok_or_else(|| NoSuchEffectOnPlant(owner_id, tile_id, effect_id))?)
    }

    pub fn take_effect(&mut self, effect_id: EffectId) -> NoSuchResult<Effect> {
        let &mut Self {
            owner_id,
            tile_id,
            ref mut effects,
            ..
        } = self;
        Ok(effects
            .iter()
            .position(|e| e.effect_id == effect_id)
            .map(|i| self.effects.swap_remove(i))
            .ok_or_else(|| NoSuchEffectOnPlant(owner_id, tile_id, effect_id))?)
    }
}

pub mod timer {
    use super::*;

    #[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
    pub enum TimerKind {
        Yield,
        Craft { recipe_index: usize },
        Rub { effect_id: EffectId },
    }

    #[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
    pub enum Lifecycle {
        // when this timer finishes, it restarts again.
        Perennial { duration: f32 },
        // this timer runs once, then, kaputt.
        Annual,
    }

    impl Lifecycle {
        pub fn is_perennial(&self) -> bool {
            matches!(self, Lifecycle::Perennial { .. } )
        }

        pub fn is_annual(&self) -> bool {
            matches!(self, Lifecycle::Annual)
        }
    }

    #[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
    pub struct Timer {
        pub until_finish: f32,
        pub lifecycle: Lifecycle,
        pub tile_id: TileId,
        pub kind: TimerKind,
    }
}
pub use timer::{Timer, TimerKind};

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Craft {
    #[serde(alias = "makes")]
    pub recipe_archetype_handle: ArchetypeHandle,
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
#[serde_diff(opaque)]
pub struct EffectId(pub uuid::Uuid);

impl fmt::Display for EffectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    /// Records whether this is the first, second, third, etc. effect to be rubbed onto this plant.
    pub effect_id: EffectId,
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
        client::{ClientError, ClientResult},
        wormhole::{ask, until_ask_id_map, AskedNote, PlantAsk},
        Ask, IdentifiesItem,
    };

    impl Plant {
        pub async fn slaughter(&self) -> ClientResult<Plant> {
            let a = Ask::Plant(PlantAsk::Slaughter {
                tile_id: self.tile_id,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantSlaughterResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantSlaughter", e))
        }

        pub async fn rub_with(&self, rub: impl IdentifiesItem) -> ClientResult<Vec<Effect>> {
            let a = Ask::Plant(PlantAsk::Rub {
                rub_item_id: rub.item_id(),
                tile_id: self.tile_id,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantRubStartResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantRub", e))
        }
        pub async fn rename(&self, new_name: String) -> ClientResult<String> {
            let a = Ask::Plant(PlantAsk::Nickname {
                tile_id: self.tile_id,
                new_name: new_name,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantRenameResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantNickname", e))
        }
 
    }
}
