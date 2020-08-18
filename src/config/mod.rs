use crate::{item, plant};
use std::{fmt, fs};

mod evalput;
pub use evalput::{Evalput, RawEvalput};

lazy_static::lazy_static! {
    pub static ref CONFIG_PATH: String = {
        std::env::var("CONFIG_PATH").unwrap_or_else(|e| {
            log::warn!("CONFIG_PATH err, defaulting to '../config'. err: {}", e);
            "../config".to_string()
        })
    };

    pub static ref CONFIG: Config = {
        RawConfig {
            plants: vec![],
            items: {
                let mut items = vec![];

                for path in walkdir::WalkDir::new(&*CONFIG_PATH)
                    .contents_first(true)
                    .into_iter()
                    .filter_map(|e| Some(e.ok()?.path().to_owned()))
                    .filter(|p| p.extension().map(|e| e == "yml" || e == "yaml").unwrap_or(false))
                {
                    let pd = path.display();
                    items.append(
                        &mut serde_yaml::from_str(
                            &fs::read_to_string(&path)
                                .unwrap_or_else(|e| panic!("Couldn't read your YAML in {}: {}", pd, e))
                        )
                        .unwrap_or_else(|e| panic!("I don't like your YAML in {}: {}", pd, e))
                    )
                }

                items
            }
        }
        .verify()
        .expect("bad")
    };
}

#[test]
fn test_lazy() {
    assert!(CONFIG.items.len() > 0);
}

#[derive(Clone)]
pub struct RawConfig {
    plants: Vec<plant::RawConfig>,
    items: Vec<item::RawConfig>,
}
impl RawConfig {
    pub fn verify(&self) -> VerifResult<Config> {
        let RawConfig { plants, items } = self.clone();
        Ok(Config {
            plants: plants.verify(self)?,
            items: items.verify(self)?,
        })
    }

    pub fn item_conf(&self, item_name: &str) -> VerifResult<item::Conf> {
        match self.items.iter().position(|i| i.name == item_name) {
            None => Err(VerifError::UnknownItem(item_name.to_owned())),
            Some(i) => Ok(item::Conf(i)),
        }
    }

    pub fn plant_conf(&self, plant_name: &str) -> VerifResult<plant::Conf> {
        match self.plants.iter().position(|i| i.name == plant_name) {
            None => Err(VerifError::UnknownPlant(plant_name.to_owned())),
            Some(i) => Ok(plant::Conf(i)),
        }
    }
}

pub struct Config {
    pub(crate) plants: Vec<plant::Config>,
    pub(crate) items: Vec<item::Config>,
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

#[derive(Debug, Clone)]
pub enum VerifError {
    UnknownItem(String),
    UnknownPlant(String),
    Custom(String),
    Constant(&'static str),
}
impl std::error::Error for VerifError {}
impl fmt::Display for VerifError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error verifying config:")?;

        use VerifError::*;
        match self {
            UnknownItem(i) => write!(
                f,
                "Config referenced item {}, but no item with this name could be found",
                i
            ),
            UnknownPlant(p) => write!(
                f,
                "Config referenced plant {}, but no plant with this name could be found",
                p
            ),
            Custom(s) => write!(f, "{}", s),
            Constant(s) => write!(f, "{}", s),
        }
    }
}
pub type VerifResult<T> = Result<T, VerifError>;

pub trait Verify {
    type Verified;

    fn verify(self, raw: &RawConfig) -> VerifResult<Self::Verified>;
}

impl<V: Verify> Verify for Vec<V> {
    type Verified = Vec<V::Verified>;

    fn verify(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.into_iter().map(|v| v.verify(raw)).collect()
    }
}

impl<V: Verify> Verify for Option<V> {
    type Verified = Option<V::Verified>;

    fn verify(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.map(|v| v.verify(raw)).transpose()
    }
}
