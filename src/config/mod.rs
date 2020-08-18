use crate::{item, plant};
use ::log::*;
use std::{fmt, fs};

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
        use serde_yaml::Value;
        use serde::de::DeserializeOwned;

        fn yml_files(folder: &str) -> impl Iterator<Item = std::path::PathBuf> {
            let path = format!("{}/{}/", &*CONFIG_PATH, folder);
            info!("\nreading {}", path);
            walkdir::WalkDir::new(path)
                .contents_first(true)
                .into_iter()
                .filter_map(|e| Some(e.ok()?.path().to_owned()))
                .filter(|p| p.extension().map(|e| e == "yml" || e == "yaml").unwrap_or(false))
        }

        fn parse_and_merge_vec<D: DeserializeOwned + fmt::Debug>(file: &str) -> Result<Vec<D>, String> {
            let values: Vec<Value> = serde_yaml::from_str(&file).map_err(|e| e.to_string())?;
            let mut output = Vec::with_capacity(values.len());
            for value in values {
                let merged = yaml_merge_keys::merge_keys_serde(value)
                    .map_err(|e| format!("merge keys {}", e))?;

                output.push(parse_merged(merged)?)
            }
            Ok(output)
        }

        fn parse_and_merge<D: DeserializeOwned + fmt::Debug>(file: &str) -> Result<D, String> {
            let value = serde_yaml::from_str(&file).map_err(|e| e.to_string())?;
            let merged = yaml_merge_keys::merge_keys_serde(value)
                .map_err(|e| format!("merge keys {}", e))?;
            parse_merged(merged)
        }

        /// Because anchors aren't officially part of the YAML spec, they're an extension,
        /// and the Rust extension is external to the YAML parsing so there's no way for it to get the line numbers,
        /// so this craziness is to resurrect line numbers in the errors to make debugging configs bearable
        fn parse_merged<D: DeserializeOwned + fmt::Debug>(merged: Value) -> Result<D, String> {
            serde_yaml::from_value(merged.clone()).map_err(|_| {
                let ser = serde_yaml::to_string(&merged).unwrap();
                let err = serde_yaml::from_str::<D>(&ser).unwrap_err();
                let err_line = match err.location() {
                    None => 0,
                    Some(l) => l.line()
                };
                let lines = ser
                    .lines()
                    .enumerate()
                    .map(|(i, l)| {
                        let mut line = l.to_string();
                        let len_before = line.len();
                        line.truncate(90);
                        if len_before > 90 {
                            line.push_str("...")
                        }
                        format!("{:>3} | {}", i, line)
                    })
                    .skip(err_line.saturating_sub(10))
                    .take(20)
                    .collect::<Vec<_>>()
                    .join("\n");
                let name = merged.get("name").or(merged.get("title")).and_then(|s| s.as_str());
                format!("\nname: {:?}\n{}\nerr: {}\n", name, lines, err)
            })
        }

        RawConfig {
            plants: {
                let mut plants = vec![];

                for path in yml_files("plants") {
                    let pd = path.display();
                    let plant_name = path.file_stem().unwrap().to_str().unwrap();

                    let skills_p = path.with_file_name(&format!("{}_skills.yml", plant_name));
                    let skills_pd = skills_p.display();
                    debug!("found {}, looking for {}", pd, skills_pd);
                    let skills: Vec<plant::RawSkill> = match fs::read_to_string(&skills_p) {
                        Ok(s) => {
                            info!("reading plant config folder at {}", pd);
                            match parse_and_merge_vec(&s) {
                                Err(e) => fatal!("I don't like your Skill YAML in {}: {}", skills_pd, e),
                                Ok(skills) => {
                                    info!("I'm happy with all {} skills in {}!", skills.len(), skills_pd);
                                    skills
                                }
                            }
                        },
                        Err(e) => {
                            debug!("couldn't read skills, {} must not be plant folder: {}", pd, e);
                            continue;
                        }
                    };

                    let file = fs::read_to_string(&path).unwrap_or_else(|e| {
                        fatal!("\nCouldn't read file {}: {}", pd, e)
                    });
                    let mut plant: plant::RawConfig = parse_and_merge(&file).unwrap_or_else(|e| {
                        fatal!("I don't like your Plant YAML in {}: {}", pd, e)
                    });

                    if plant.skills.len() > 0 {
                        fatal!("plant skills must be defined in external file");
                    }
                    plant.skills = skills;
                    plants.push(plant)
                }

                plants
            },
            items: {
                let mut items = vec![];

                for path in yml_files("items") {
                    let pd = path.display();
                    let file = fs::read_to_string(&path).unwrap_or_else(|e| {
                        fatal!("\nCouldn't read file {}: {}", pd, e)
                    });
                    let mut contents: Vec<item::RawConfig> = parse_and_merge_vec(&file)
                        .unwrap_or_else(|e| fatal!("I don't like your YAML in {}: {}", pd, e));
                    info!("I like all {} items in {}!", contents.len(), pd);
                    items.append(&mut contents);
                }

                items
            }
        }
        .verify()
        .unwrap_or_else(|e| fatal!("I ran into trouble verifying your config: {}", e))
    };
}

#[test]
fn test_lazy() {
    drop(pretty_env_logger::try_init());

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
