use crate::{hackstead, item, plant};
use log::*;

#[cfg(feature = "config_verify")]
mod verify;
#[cfg(feature = "config_verify")]
pub use verify::{
    yaml_and_verify, FromFile, RawConfig, VerifError, VerifNote, VerifResult, Verify,
};

/// The kind of map you should look up your Confs in.
pub type ConfMap<K, V> = std::collections::HashMap<K, V>;

pub mod evalput;
pub use evalput::Evalput;
#[cfg(feature = "config_verify")]
pub use evalput::RawEvalput;

lazy_static::lazy_static! {
    pub static ref CONFIG_PATH: String = {
        std::env::var("CONFIG_PATH").unwrap_or_else(|e| {
            warn!("CONFIG_PATH err, defaulting to '../config'. err: {}", e);
            "../config".to_string()
        })
    };

    pub static ref CONFIG: Config = {
        let path = format!("{}/config.bincode", &*CONFIG_PATH);
        bincode::deserialize(
            zstd::decode_all(
                std::fs::read(&path)
                    .unwrap_or_else(|e| panic!("opening {}: {}", path, e))
                    .as_slice()
            )
            .unwrap_or_else(|e| panic!("couldn't decompress config: {}", e))
            .as_slice()
        )
        .unwrap_or_else(|e| panic!("parsing {}: {}", path, e))
    };
}

pub struct LevelInfo {
    pub xp_so_far: usize,
    pub xp_to_go: usize,
    pub total_level_xp: usize,
    pub last_unlocked_index: usize,
}

pub fn max_level_info(
    mut your_xp: usize,
    mut level_xps: impl ExactSizeIterator<Item = usize>,
) -> LevelInfo {
    let mut xp_so_far = 0;
    let mut xp_to_go = 0;
    let mut total_level_xp = 0;

    let level_count = level_xps.len();
    let last_unlocked_index = level_xps
        .position(|xp| match your_xp.checked_sub(xp) {
            None => {
                xp_to_go = your_xp;
                xp_so_far = xp - your_xp;
                total_level_xp = xp;
                true
            }
            Some(subbed_xp) => {
                your_xp = subbed_xp;
                false
            }
        })
        .unwrap_or(level_count);

    LevelInfo {
        xp_so_far,
        xp_to_go,
        total_level_xp,
        last_unlocked_index,
    }
}

pub fn max_level_index(your_xp: usize, level_xps: impl ExactSizeIterator<Item = usize>) -> usize {
    max_level_info(your_xp, level_xps).last_unlocked_index
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub plants: ConfMap<plant::Conf, plant::Config>,
    pub items: ConfMap<item::Conf, item::Config>,
    pub hackstead: hackstead::Config,
}
impl Config {
    pub fn item_named(&self, item_name: &str) -> Option<&item::Config> {
        self.items.values().find(|i| i.name == item_name)
    }

    pub fn welcome_gifts(&self) -> impl Iterator<Item = &item::Config> {
        self.items.values().filter(|a| a.welcome_gift)
    }

    pub fn seeds(&self) -> impl Iterator<Item = (plant::Conf, &item::Config)> {
        self.items.values().filter_map(|c| Some((c.grows_into?, c)))
    }

    pub fn recipes(&self) -> impl Iterator<Item = &plant::Recipe> {
        self.plants
            .values()
            .flat_map(|p| p.skills.values().flat_map(|s| s.effects.iter()))
            .filter_map(|e| e.kind.buff())
            .flat_map(|buff| buff.recipes())
    }

    pub fn land_unlockers(&self) -> impl Iterator<Item = (&item::LandUnlock, &item::Config)> {
        self.items
            .values()
            .filter_map(|c| Some((c.unlocks_land.as_ref()?, c)))
    }
}
