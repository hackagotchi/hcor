use crate::{hackstead, item, plant};
use log::*;

#[cfg(feature = "config_verify")]
mod verify;
#[cfg(feature = "config_verify")]
pub use verify::{
    yaml_and_verify, FromFile, RawConfig, VerifError, VerifNote, VerifResult, Verify,
};

mod evalput;
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
        let path = format!("{}/config.json", &*CONFIG_PATH);
        serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("opening {}: {}", path, e))
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

pub fn max_level_info(mut your_xp: usize, mut level_xps: impl Iterator<Item = usize>) -> LevelInfo {
    let mut xp_so_far = 0;
    let mut xp_to_go = 0;
    let mut total_level_xp = 0;

    let last_unlocked_index = level_xps
        .position(|xp| match your_xp.checked_sub(xp) {
            None | Some(0) => {
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
        .and_then(|p| p.checked_sub(0))
        .unwrap_or(0);

    LevelInfo {
        xp_so_far,
        xp_to_go,
        total_level_xp,
        last_unlocked_index,
    }
}

pub fn max_level_index(your_xp: usize, level_xps: impl Iterator<Item = usize>) -> usize {
    max_level_info(your_xp, level_xps).last_unlocked_index
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub(crate) plants: Vec<plant::Config>,
    pub(crate) items: Vec<item::Config>,
    pub(crate) hackstead: hackstead::Config,
}
impl Config {
    pub fn welcome_gifts(&self) -> impl Iterator<Item = &item::Config> {
        self.items.iter().filter(|a| a.welcome_gift)
    }

    pub fn seeds(&self) -> impl Iterator<Item = (plant::Conf, &item::Config)> {
        self.items.iter().filter_map(|c| Some((c.grows_into?, c)))
    }

    pub fn land_unlockers(&self) -> impl Iterator<Item = (&item::LandUnlock, &item::Config)> {
        self.items
            .iter()
            .filter_map(|c| Some((c.unlocks_land.as_ref()?, c)))
    }
}
