#![feature(try_trait)]
#![warn(missing_docs)]

mod errors;
pub use errors::ServiceError;
#[cfg(feature="mongo")]
pub use errors::RequestError;

pub mod config;
pub use config::CONFIG;

pub mod item;
pub use item::Item;

pub mod hackstead;
pub use hackstead::Hackstead;

pub mod user_contact;
pub use user_contact::UserContact;

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
