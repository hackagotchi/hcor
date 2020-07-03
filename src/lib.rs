#![feature(try_trait)]
//#![warn(missing_docs)]

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

pub mod frontend {
    pub fn emojify<S: ToString>(txt: S) -> String {
        format!(":{}:", txt.to_string().replace(" ", "_"))
    }
}
