use crate::config::{ArchetypeHandle, PlantArchetype};
use crate::*;
use serde::Serialize;
use std::time::SystemTime;
use tokio_postgres::Client as PgClient;
use tokio_postgres::error::Error as SqlError;


#[derive(Clone, Debug, Serialize)]
pub struct Hacksteader {
    pub user_id: String,
    pub profile: Profile,
    pub land: Vec<Tile>,
    pub inventory: Vec<Possession>,
    pub gotchis: Vec<Possessed<possess::Gotchi>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    pub acquired: SystemTime,
    pub plant: Option<Plant>,
    pub id: uuid::Uuid,
    pub steader: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Profile {
    /// Indicates when this Hacksteader first joined the elite community.
    pub joined: SystemTime,
    pub last_active: SystemTime,
    pub last_farm: SystemTime,
    /// This is not an uuid::Uuid because it's actually the steader id of the person who owns this Profile
    pub id: String,
    pub xp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plant {
    pub xp: u64,
    pub until_yield: f32,
    pub craft: Option<Craft>,
    pub pedigree: Vec<possess::seed::SeedGrower>,
    pub archetype_handle: ArchetypeHandle,
}

#[derive(Debug, Clone, Serialize)]
pub struct Craft {
    pub until_finish: f32,
    pub total_cycles: f32,
    pub destroys_plant: bool,
    pub makes: ArchetypeHandle,
}

impl Hacksteader {
    pub async fn insert_to_sql(&self, client: &PgClient) -> Result<(), SqlError> {

        Ok(())
    }

    pub async fn update_in_sql(&self, client: &PgClient) -> Result<(), SqlError> {
        Ok(())
    }
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
