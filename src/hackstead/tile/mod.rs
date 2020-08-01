pub mod plant;
pub use plant::Plant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a user's item is consumed in exchange for a new tile of land.
/// The steader to give the land to is inferred to be the owner of the item.
pub struct TileCreationRequest {
    /// id for an item that is capable of being removed in exchange for another tile of land.
    pub tile_redeemable_item_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TileBase {
    pub owner_id: Uuid,
    pub tile_id: Uuid,
    pub acquired: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tile {
    pub plant: Option<Plant>,
    pub base: TileBase,
}
impl Tile {
    pub fn new(owner_id: Uuid) -> Tile {
        Tile {
            plant: None,
            base: TileBase {
                acquired: Utc::now(),
                tile_id: Uuid::new_v4(),
                owner_id,
            },
        }
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::client::{client, ClientResult, IdentifiesItem, SERVER_URL};
    use plant::Plant;

    impl Tile {
        pub async fn plant_seed(&self, seed: impl IdentifiesItem) -> ClientResult<Plant> {
            Ok(client()
                .post(&format!("{}/{}", *SERVER_URL, "plant/new"))
                .send_json(&plant::PlantCreationRequest {
                    seed_item_id: seed.item_id(),
                    tile_id: self.base.tile_id,
                })
                .await?
                .json()
                .await?)
        }
    }
}
