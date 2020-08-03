#![feature(try_trait)]
//#![warn(missing_docs)]

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
pub use wormhole::Note;
#[cfg(feature = "client")]
pub use wormhole::Wormhole;

/// How many times per second the server should update.
///
/// TODO: these should probably go in the config
pub const UPDATES_PER_SECOND: u64 = 20;
/// An instance of Duration representing the same thing as UPDATES_PER_SECOND.
pub const UPDATE_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(1000 / UPDATES_PER_SECOND);

/// All of the game design switches and levers are handled here, with a focus on how they interact
/// with the rest of the data in the game.
pub mod config;
pub use config::{ConfigError, ConfigResult, CONFIG};

/// What are those Hackagotchi farms made of, anyway?
pub mod hackstead;
pub use hackstead::{item, plant, tile, Hackstead, Item, Plant, Tile};

/// Identification convienience traits and our very own `UserId` enum.
pub mod id;
pub use id::{
    IdentifiesItem, IdentifiesPlant, IdentifiesSteader, IdentifiesTile, IdentifiesUser, UserId,
};

/// Contains code common across frontends.
pub mod frontend {
    /// Takes the name of something and reformats it such that a text preformatter should be able
    /// to recognize and replace it with an emoji.
    pub fn emojify<S: ToString>(txt: S) -> String {
        format!(":{}:", txt.to_string().replace(" ", "_"))
    }
}
