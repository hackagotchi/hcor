use crate::{SteaderId, TileId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

pub mod plant;
pub use plant::Plant;

#[derive(Clone, Debug, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Tile {
    pub plant: Option<Plant>,
    pub owner_id: SteaderId,
    pub tile_id: TileId,
    #[serde_diff(opaque)]
    pub acquired: DateTime<Utc>,
}
impl Tile {
    pub fn new(owner_id: SteaderId) -> Tile {
        Tile {
            plant: None,
            acquired: Utc::now(),
            tile_id: TileId(uuid::Uuid::new_v4()),
            owner_id,
        }
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

    impl Tile {
        pub async fn plant_seed(&self, seed: impl IdentifiesItem) -> ClientResult<Plant> {
            let a = Ask::Plant(PlantAsk::Summon {
                seed_item_id: seed.item_id(),
                tile_id: self.tile_id,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantSummonResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantSummon", e))
        }
    }
}
