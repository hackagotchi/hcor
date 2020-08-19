use crate::{item, plant};
use ::log::*;
use std::fmt;

mod parse;

mod evalput;
pub use evalput::{Evalput, RawEvalput};

lazy_static::lazy_static! {
    pub static ref CONFIG_PATH: String = {
        std::env::var("CONFIG_PATH").unwrap_or_else(|e| {
            warn!("CONFIG_PATH err, defaulting to '../config'. err: {}", e);
            "../config".to_string()
        })
    };

    pub static ref CONFIG: Config = {
        let plants = parse::read_plants();
        let items = parse::read_items();
        RawConfig {
            plant_name_corpus: ngrammatic::CorpusBuilder::new()
                .fill(plants.iter().map(|p| p.name.as_ref()))
                .finish(),
            plants,
            item_name_corpus: ngrammatic::CorpusBuilder::new()
                .fill(items.iter().map(|i| i.name.as_ref()))
                .finish(),
            items,
        }
        .verify()
        .unwrap_or_else(|e| fatal!("{}", e))
    };
}

#[test]
fn test_lazy() {
    drop(pretty_env_logger::try_init());

    assert!(CONFIG.items.len() > 0);
}

pub struct RawConfig {
    plants: Vec<plant::RawConfig>,
    plant_name_corpus: ngrammatic::Corpus,
    items: Vec<item::RawConfig>,
    item_name_corpus: ngrammatic::Corpus,
}
impl RawConfig {
    pub fn verify(&self) -> VerifResult<Config> {
        let RawConfig { plants, items, .. } = self;
        Ok(Config {
            plants: plants.clone().verify(self)?,
            items: items.clone().verify(self)?,
        })
    }

    pub fn item_conf(&self, item_name: &str) -> VerifResult<item::Conf> {
        match self.items.iter().position(|i| i.name == item_name) {
            None => Err(VerifError {
                kind: VerifErrorKind::UnknownItem(
                    item_name.to_owned(),
                    self.item_name_corpus.search(item_name, 0.2),
                ),
                source: vec![],
            }),
            Some(i) => Ok(item::Conf(i)),
        }
    }

    pub fn plant_conf(
        &self,
        plant_name: &str,
    ) -> VerifResult<plant::Conf> {
        match self.plants.iter().position(|i| i.name == plant_name) {
            None => Err(VerifError {
                kind: VerifErrorKind::UnknownPlant(
                    plant_name.to_owned(),
                    self.plant_name_corpus.search(plant_name, 0.2),
                ),
                source: vec![],
            }),
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
pub enum VerifErrorKind {
    UnknownItem(String, Vec<ngrammatic::SearchResult>),
    UnknownPlant(String, Vec<ngrammatic::SearchResult>),
    Custom(String),
}
impl fmt::Display for VerifErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use VerifErrorKind::*;
        match self {
            UnknownItem(i, sr) => write!(
                f,
                "Config referenced item {}, \
                    but no item with this name could be found. \
                    Perhaps you meant one of: {}?",
                i,
                sr.into_iter()
                    .map(|s| s.text.as_ref())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            UnknownPlant(p, sr) => write!(
                f,
                "Config referenced plant {}, \
                    but no plant with this name could be found. \
                    Perhaps you meant one of: {}?",
                p,
                sr.into_iter()
                    .map(|s| s.text.as_ref())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerifError {
    kind: VerifErrorKind,
    source: Vec<String>,
}
impl VerifError {
    pub fn custom(s: impl AsRef<str>) -> VerifError {
        VerifError {
            kind: VerifErrorKind::Custom(s.as_ref().to_owned()),
            source: vec![],
        }
    }
}
impl fmt::Display for VerifError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "I ran into trouble verifying your config \n{}\nas {}",
            self.source
                .iter()
                .rev()
                .map(|s| format!("> {}", s.to_string()))
                .collect::<Vec<_>>()
                .join("\n"),
            self.kind
        )
    }
}
impl std::error::Error for VerifError {}
pub type VerifResult<T> = Result<T, VerifError>;

pub trait Verify: Sized {
    type Verified;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified>;

    fn context(&self) -> String;

    fn verify(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        let context = self.context().clone();
        self.verify_raw(raw).map_err(|mut e| {
            println!("pushing {}", context);
            e.source.push(context);
            e
        })
    }
}

impl<V: Verify> Verify for Vec<V> {
    type Verified = Vec<V::Verified>;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.into_iter().map(|v| v.verify(raw)).collect()
    }

    fn context(&self) -> String {
        match self.first() {
            Some(v) => v.context(),
            None => String::from("Unknown"),
        }
    }
}

impl<V: Verify> Verify for Option<V> {
    type Verified = Option<V::Verified>;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.map(|v| v.verify(raw)).transpose()
    }

    fn context(&self) -> String {
        match self {
            Some(v) => v.context(),
            None => String::from("Unknown"),
        }
    }
}
