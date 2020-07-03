#![feature(try_trait)]
#![warn(missing_docs)]

mod errors;
#[cfg(feature = "mongo")]
pub use errors::RequestError;
pub use errors::ServiceError;

/// All of the game design switches and levers are handled here, with a focus on how they interact
/// with the rest of the data in the game.
pub mod config;
pub use config::CONFIG;

/// Some items boost the growth of plants; others accelerate their growth or give you more land.
/// This module facilitates handling all of them.
pub mod item;
pub use item::Item;

/// What are those Hackagotchi farms made of, anyway?
pub mod hackstead;
pub use hackstead::Hackstead;

/// Store user emails/slack ids with a compile time check that we'll have at least one of those
/// two.
pub mod user_contact;
pub use user_contact::UserContact;

/// Addresses Hackagotchi's market platform.
pub mod market;
pub use market::Sale;

/// Contains code common across frontends.
pub mod frontend {
    /// Takes the name of something and reformats it such that a text preformatter should be able
    /// to recognize and replace it with an emoji.
    pub fn emojify<S: ToString>(txt: S) -> String {
        format!(":{}:", txt.to_string().replace(" ", "_"))
    }
}
