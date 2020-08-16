//#![warn(missing_docs)]

/// re-export this dependency to make it easier to sync versions
pub use serde_diff;

// All of the game design switches and levers are handled here, with a focus on how they interact
// with the rest of the data in the game.
pub mod config;
//pub use config::{ConfigError, ConfigResult, CONFIG};

/*
#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{ClientError, ClientResult};
#[cfg(feature = "client")]
/// This is exposed to aid those extending hcor's wrappers around the API.
pub mod client_internal {
    pub use super::client::request;
}
/// The Wormhole allows quick communication with the server,
/// used especially for receiving information about game events as soon as they occur.
pub mod wormhole;
#[cfg(feature = "client")]
pub use wormhole::WormholeResult;
pub use wormhole::{Ask, Note};

lazy_static::lazy_static! {
    /// How many times per second the server should update.
    pub static ref UPDATES_PER_SECOND: u64 = {
        std::env::var("UPDATES_PER_SECOND")
            .map_err(|e| e.to_string())
            .and_then(|x| x.parse::<u64>().map_err(|e| e.to_string()))
            .unwrap_or_else(|e| {
                log::warn!("UPDATES_PER_SECOND err, defaulting to 20. err: {}", e);
                20
            })
    };
    /// An instance of Duration representing the same thing as UPDATES_PER_SECOND.
    pub static ref UPDATE_INTERVAL: std::time::Duration =
        std::time::Duration::from_nanos(1e9 as u64 / *UPDATES_PER_SECOND);
}

/// What are those Hackagotchi farms made of, anyway?
pub mod hackstead;
pub use hackstead::{item, plant, tile, Hackstead, Item, Plant, Tile};

/// Identification convienience traits and our very own `UserId` enum.
pub mod id;
pub use id::{
    IdentifiesItem, IdentifiesPlant, IdentifiesSteader, IdentifiesTile, IdentifiesUser, ItemId,
    SteaderId, TileId, UserId,
};

/// Contains code common across frontends.
pub mod frontend {
    /// Takes the name of something and reformats it such that a text preformatter should be able
    /// to recognize and replace it with an emoji.
    pub fn emojify<S: ToString>(txt: S) -> String {
        format!(":{}:", txt.to_string().replace(" ", "_"))
    }
}*/
