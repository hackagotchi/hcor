use super::{Config, CONFIG_PATH};
use crate::{hackstead, item, plant};
use log::*;
use std::fmt;

mod parse;

pub fn yaml_and_verify() -> Result<Config, String> {
    let plants = parse::read_plants()?;
    let items = parse::read_items()?;
    let hackstead = parse::read::<hackstead::Config>("hackstead")?;
    info!("I like all {} advancements in hackstead.yml!", hackstead.advancements.len());

    RawConfig {
        plant_name_corpus: ngrammatic::CorpusBuilder::new()
            .fill(plants.iter().map(|p| p.name.as_ref()))
            .finish(),
        plant_skill_title_corpuses: plants
            .iter()
            .map(|p| {
                (
                    p.conf,
                    ngrammatic::CorpusBuilder::new()
                        .fill(p.skills.iter().map(|s| s.title.as_ref()))
                        .finish(),
                )
            })
            .collect(),
        plants,
        item_name_corpus: ngrammatic::CorpusBuilder::new()
            .fill(items.iter().map(|i| i.name.as_ref()))
            .finish(),
        items,
        hackstead,
    }
    .verify()
    .map_err(|e| format!("{}", e))
}

pub struct RawConfig {
    pub plants: Vec<FromFile<plant::RawConfig>>,
    pub plant_name_corpus: ngrammatic::Corpus,
    pub plant_skill_title_corpuses: std::collections::HashMap<plant::Conf, ngrammatic::Corpus>,
    pub items: Vec<FromFile<item::RawConfig>>,
    pub item_name_corpus: ngrammatic::Corpus,
    pub hackstead: FromFile<hackstead::Config>,
}
impl Default for RawConfig {
    fn default() -> Self {
        use ngrammatic::CorpusBuilder;

        Self {
            plants: vec![],
            plant_name_corpus: CorpusBuilder::new().finish(),
            plant_skill_title_corpuses: Default::default(),
            items: vec![],
            item_name_corpus: CorpusBuilder::new().finish(),
            hackstead: FromFile::new(Default::default(), "unknown file".to_string()),
        }
    }
}
impl RawConfig {
    pub fn verify(&self) -> VerifResult<Config> {
        let RawConfig {
            hackstead,
            plants,
            items,
            ..
        } = self;
        Ok(Config {
            plants: plants.clone().verify(self)?,
            items: items.clone().verify(self)?,
            hackstead: hackstead.inner.clone(),
        })
    }

    pub fn item_conf(&self, item_name: &str) -> VerifResult<item::Conf> {
        match self.items.iter().find(|i| i.name == item_name) {
            None => Err(VerifError::new(VerifErrorKind::UnknownItem(
                item_name.to_owned(),
                self.item_name_corpus.search(item_name, 0.35),
            ))),
            Some(i) => Ok(i.conf),
        }
    }

    pub fn plant_skill_conf(
        &self,
        plant_conf: plant::Conf,
        skill_title: &str,
    ) -> VerifResult<plant::skill::Conf> {
        match self
            .plant(plant_conf)
            .skills
            .iter()
            .find(|s| s.title == skill_title)
        {
            None => Err(VerifError::new(VerifErrorKind::UnknownPlantSkill(
                plant_conf,
                skill_title.to_string(),
                self.plant_skill_title_corpuses[&plant_conf].search(skill_title, 0.35),
            ))),
            Some(s) => Ok(plant::skill::Conf(plant_conf, s.conf)),
        }
    }

    pub fn plant_conf(&self, plant_name: &str) -> VerifResult<plant::Conf> {
        match self.plants.iter().find(|i| i.name == plant_name) {
            None => Err(VerifError::new(VerifErrorKind::UnknownPlant(
                plant_name.to_owned(),
                self.plant_name_corpus.search(plant_name, 0.2),
            ))),
            Some(p) => Ok(p.conf),
        }
    }

    pub fn plant(&self, conf: plant::Conf) -> &plant::RawConfig {
        self.plants.iter().find(|p| p.conf == conf).unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum VerifErrorKind {
    UnknownItem(String, Vec<ngrammatic::SearchResult>),
    UnknownPlant(String, Vec<ngrammatic::SearchResult>),
    UnknownPlantSkill(plant::Conf, String, Vec<ngrammatic::SearchResult>),
    Custom(String),
}
impl fmt::Display for VerifErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use VerifErrorKind::*;
        match self {
            UnknownItem(i, sr) => write!(
                f,
                "referenced item {}, \
                    but no item with this name could be found. \
                    Perhaps you meant {}?",
                i,
                sr.into_iter()
                    .map(|s| s.text.as_ref())
                    .collect::<Vec<_>>()
                    .join(", or "),
            ),
            UnknownPlantSkill(pc, s, sr) => write!(
                f,
                "referenced skill {} on plant conf {}, \
                    but no skill with this name on this plant could be found. \
                    Perhaps you meant {}?",
                s,
                pc.0,
                sr.into_iter()
                    .map(|s| s.text.as_ref())
                    .collect::<Vec<_>>()
                    .join(", or "),
            ),
            UnknownPlant(p, sr) => write!(
                f,
                "referenced plant {}, \
                    but no plant with this name could be found. \
                    Perhaps you meant {}?",
                p,
                sr.into_iter()
                    .map(|s| s.text.as_ref())
                    .collect::<Vec<_>>()
                    .join(", or "),
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
    pub fn custom(s: impl AsRef<str>) -> Self {
        VerifError::new(VerifErrorKind::Custom(s.as_ref().to_owned()))
    }

    pub fn new(kind: VerifErrorKind) -> Self {
        VerifError {
            kind,
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

pub trait VerifNote {
    fn note(self, context: impl AsRef<str>) -> Self;
}

impl<T> VerifNote for VerifResult<T> {
    fn note(self, context: impl AsRef<str>) -> Self {
        self.map_err(|mut e| {
            e.source.push(context.as_ref().to_string());
            e
        })
    }
}

pub trait Verify: Sized {
    type Verified;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified>;

    fn context(&self) -> Option<String>;

    fn verify(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        let context = self.context().clone();
        self.verify_raw(raw).map_err(|mut e| {
            if let Some(c) = context {
                e.source.push(c)
            }
            e
        })
    }
}

impl<V: Verify> Verify for Vec<V> {
    type Verified = Vec<V::Verified>;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.into_iter().map(|v| v.verify(raw)).collect()
    }

    fn context(&self) -> Option<String> {
        None
        /*
        Some(match self.first().and_then(|e| e.context()) {
            Some(v) => format!("in a list beginning {}", v),
            None => "in an empty list".to_string(),
        })*/
    }
}

impl<V: Verify> Verify for Option<V> {
    type Verified = Option<V::Verified>;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.map(|v| v.verify(raw)).transpose()
    }

    fn context(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FromFile<V> {
    pub inner: V,
    #[serde(skip)]
    pub file: String,
}
impl<V> FromFile<V> {
    pub fn new(inner: V, file: String) -> Self {
        Self { inner, file }
    }

    pub fn map<T>(self, f: impl FnOnce(V) -> T) -> FromFile<T> {
        FromFile {
            inner: f(self.inner),
            file: self.file,
        }
    }
}
impl<V> std::ops::Deref for FromFile<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<V> std::ops::DerefMut for FromFile<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<V: Verify> Verify for FromFile<V> {
    type Verified = V::Verified;

    fn verify_raw(self, raw: &RawConfig) -> VerifResult<Self::Verified> {
        self.inner.verify(raw)
    }

    fn context(&self) -> Option<String> {
        Some(format!("from a file {}", self.file))
    }
}
