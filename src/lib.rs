#![feature(try_trait)]
//#![warn(missing_docs)]

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{ClientError, ClientResult, IdentifiesItem, IdentifiesSteader, IdentifiesUser};
#[cfg(feature = "client")]
/// This is exposed to aid those extending hcor's wrappers around the API.
pub mod client_internal {
    pub use super::client::{client, SERVER_URL};
}

/// All of the game design switches and levers are handled here, with a focus on how they interact
/// with the rest of the data in the game.
pub mod config;
pub use config::{ConfigError, ConfigResult, CONFIG};

/// What are those Hackagotchi farms made of, anyway?
pub mod hackstead;
pub use hackstead::{item, plant, tile, Hackstead, Item, Plant, Tile};

/// Store user emails/slack ids with a compile time check that we'll have at least one of those
/// two.
pub mod user_id;
pub use user_id::UserId;

/// Contains code common across frontends.
pub mod frontend {
    /// Takes the name of something and reformats it such that a text preformatter should be able
    /// to recognize and replace it with an emoji.
    pub fn emojify<S: ToString>(txt: S) -> String {
        format!(":{}:", txt.to_string().replace(" ", "_"))
    }
}
