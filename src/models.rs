use crate::config::{ArchetypeHandle, PlantArchetype};
use crate::*;
use serde::Serialize;
use time::PrimitiveDateTime;
use sqlx::types::Type;
use sqlx::FromRow;


#[derive(Clone, Debug, Serialize, FromRow)]
pub struct Hacksteader {
    pub user_id: String,
    pub profile: Profile,
    pub land: Vec<Tile>,
    pub inventory: Vec<Possession>,
    pub gotchis: Vec<Possessed<possess::Gotchi>>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Tile {
    pub acquired: PrimitiveDateTime,
    pub plant: Option<Plant>,
    pub id: uuid::Uuid,
    pub steader: String,
}

#[derive(Clone, Debug, Serialize, Type)]
pub struct Profile {
    /// Indicates when this Hacksteader first joined the elite community.
    pub joined: PrimitiveDateTime,
    pub last_active: PrimitiveDateTime,
    pub last_farm: PrimitiveDateTime,
    /// This is not an uuid::Uuid because it's actually the steader id of the person who owns this Profile
    pub id: String,
    pub xp: u32,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Plant {
    pub xp: u32,
    pub until_yield: f32,
    pub craft: Option<Craft>,
    pub pedigree: Vec<possess::seed::SeedGrower>,
    pub archetype_handle: ArchetypeHandle,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Craft {
    pub until_finish: f32,
    pub total_cycles: f32,
    pub destroys_plant: bool,
    pub makes: ArchetypeHandle,
}

impl std::ops::Deref for Plant {
    type Target = PlantArchetype;

    fn deref(&self) -> &Self::Target {
        &CONFIG
            .plant_archetypes
            .get(self.archetype_handle)
            .expect("invalid archetype handle")
    }
}
