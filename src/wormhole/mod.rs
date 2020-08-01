use crate::Item;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::Wormhole;

/// How often heartbeat pings are sent
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
/// How long before lack of client response causes a timeout
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long before lack of server response causes a timeout
pub const SERVER_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Serialize, Deserialize)]
pub enum Note {
    Yield { items: Vec<Item>, xp: i32 },
    CraftFinish { items: Vec<Item>, xp: i32 },
}
