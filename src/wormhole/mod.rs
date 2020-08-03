use crate::{plant, Item};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{Wormhole, WormholeError};

/// How often heartbeat pings are sent
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
/// How long before lack of client response causes a timeout
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long before lack of server response causes a timeout
pub const SERVER_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a wormhole connection be established
pub struct EstablishWormholeRequest {
    /// The uuid of the user to be associated with this wormhole connection;
    /// only events relevant to this user will be transferred through.
    pub user_id: crate::UserId,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// Like a notification, but cuter; a tidbit of information that the server
/// thinks you might have a special interest in.
pub enum Note {
    YieldProgress {
        until_finish: f64,
        tile_id: Uuid,
    },
    YieldFinish {
        items: Vec<Item>,
        xp: i32,
        tile_id: Uuid,
    },
    CraftFinish {
        items: Vec<Item>,
        xp: i32,
        tile_id: Uuid,
    },
    CraftProgress {
        until_finish: f64,
        tile_id: Uuid,
    },
    PlantEffectFinish {
        effect: plant::Effect,
        tile_id: Uuid,
    },
    PlantEffectProgress {
        until_finish: f64,
        tile_id: Uuid,
        rub_index: i32,
    },
}
