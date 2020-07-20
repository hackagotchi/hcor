use crate::{config, item};
use config::{ArchetypeHandle, PlantArchetype, CONFIG};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting than an item to be applied to a plant
pub struct PlantApplicationRequest {
    /// the item to be applied to a plant
    pub applicable_item_id: uuid::Uuid,
    /// the tile that the plant to apply this to rests on
    pub tile_id: uuid::Uuid,
    /// the steader who owns the item, plant and tile
    pub steader: crate::UserContact,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting than a plant be removed from its tile
pub struct PlantRemovalRequest {
    /// The tile that the plant the user wants to remove sits on
    pub tile_id: uuid::Uuid,
    /// The hacksteader who owns the plant and tile
    pub steader: crate::UserContact,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a plant begin crafting something.
pub struct PlantCraftRequest {
    /// The tile that the plant that should craft sits on
    pub tile_id: uuid::Uuid,
    /// The index of the recipe in the list of this plant's recipes
    pub recipe_index: usize
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Plant {
    #[serde(with = "bson::compat::u2f")]
    pub xp: u64,
    pub until_yield: f32,
    pub craft: Option<Craft>,
    pub pedigree: Vec<item::seed::SeedGrower>,
    /// Effects from potions, warp powder, etc. that actively change the behavior of this plant.
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(with = "bson::compat::u2f")]
    pub archetype_handle: ArchetypeHandle,
    /// This field isn't saved to the database, and is just used
    /// when `plant.increase_xp()` is called.
    #[serde(default, with = "bson::compat::u2f")]
    pub queued_xp_bonus: u64,
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
    pub fn from_seed(seed: item::Seed) -> Self {
        let mut s = Self {
            archetype_handle: CONFIG.find_plant_handle(&seed.grows_into).unwrap(),
            pedigree: seed.pedigree,
            ..Default::default()
        };
        s.until_yield = s.base_yield_duration.unwrap_or(0.0);
        s
    }

    fn effect_advancements<'a>(&'a self) -> impl Iterator<Item = &'a config::PlantAdvancement> {
        self.effects
            .iter()
            .filter_map(|e| {
                CONFIG.get_item_application_plant_advancement(
                    e.item_archetype_handle,
                    e.effect_archetype_handle,
                )
            })
            .map(|(_effect, adv)| adv)
    }

    /// Excludes neighbor bonuses
    pub fn advancements_sum<'a>(
        &'a self,
        extra_advancements: impl Iterator<Item = &'a config::PlantAdvancement>,
    ) -> config::PlantAdvancementSum {
        self.advancements.sum(
            self.xp,
            self.effect_advancements().chain(extra_advancements),
        )
    }

    /// A sum struct for all of the possible advancements for this plant,
    /// plus any effects it has active.
    pub fn advancements_max_sum<'a>(
        &'a self,
        extra_advancements: impl Iterator<Item = &'a config::PlantAdvancement>,
    ) -> config::PlantAdvancementSum {
        self.advancements
            .max(self.effect_advancements().chain(extra_advancements))
    }

    pub fn neighborless_advancements_sum<'a>(
        &'a self,
        extra_advancements: impl Iterator<Item = &'a config::PlantAdvancement>,
    ) -> config::PlantAdvancementSum {
        self.advancements.raw_sum(
            self.xp,
            self.effect_advancements().chain(extra_advancements),
        )
    }

    pub fn unlocked_advancements<'a>(
        &'a self,
        extra_advancements: impl Iterator<Item = &'a config::PlantAdvancement>,
    ) -> impl Iterator<Item = &'a config::PlantAdvancement> {
        self.advancements
            .unlocked(self.xp)
            .chain(self.effect_advancements())
            .chain(extra_advancements)
    }

    pub fn all_advancements<'a>(
        &'a self,
        extra_advancements: impl Iterator<Item = &'a config::PlantAdvancement>,
    ) -> impl Iterator<Item = &'a config::PlantAdvancement> {
        self.advancements
            .all()
            .chain(self.effect_advancements())
            .chain(extra_advancements)
    }

    pub fn current_advancement(&self) -> &config::PlantAdvancement {
        self.advancements.current(self.xp)
    }

    pub fn next_advancement(&self) -> Option<&config::PlantAdvancement> {
        self.advancements.next(self.xp)
    }

    pub fn increase_xp(&mut self, mut amt: u64) -> Option<&'static config::PlantAdvancement> {
        amt += self.queued_xp_bonus;
        self.queued_xp_bonus = 0;
        CONFIG
            .plant_archetypes
            .get(self.archetype_handle as usize)
            .expect("invalid archetype handle")
            .advancements
            .increase_xp(&mut self.xp, amt)
    }

    pub fn current_recipe_raw(&self) -> Option<config::Recipe<ArchetypeHandle>> {
        self.craft
            .as_ref()
            .and_then(|c| self.get_recipe_raw(c.recipe_archetype_handle))
    }

    pub fn current_recipe(&self) -> Option<config::Recipe<&'static config::Archetype>> {
        self.current_recipe_raw().and_then(|x| x.lookup_handles())
    }

    pub fn get_recipe_raw(
        &self,
        recipe_ah: ArchetypeHandle,
    ) -> Option<config::Recipe<ArchetypeHandle>> {
        self.advancements_sum(std::iter::empty())
            .recipes
            .get(recipe_ah as usize)
            .cloned()
    }

    pub fn get_recipe(
        &self,
        recipe_ah: ArchetypeHandle,
    ) -> Option<config::Recipe<&'static config::Archetype>> {
        self.get_recipe_raw(recipe_ah)
            .and_then(|x| x.lookup_handles())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Craft {
    pub until_finish: f32,
    #[serde(default, with = "bson::compat::u2f")]
    pub recipe_archetype_handle: ArchetypeHandle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    pub until_finish: Option<f32>,
    /// The archetype of the item that was consumed to apply this effect.
    #[serde(default, with = "bson::compat::u2f")]
    pub item_archetype_handle: ArchetypeHandle,
    /// The archetype of the effect within this item that describes this effect.
    #[serde(default, with = "bson::compat::u2f")]
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
