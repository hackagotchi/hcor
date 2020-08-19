use crate::{
    config,
    id::{NoSuchEffectOnPlant, NoSuchResult},
    item, IdentifiesSteader, IdentifiesTile, SteaderId, TileId,
};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;

mod buff;
pub use buff::{Buff, RawBuff, RawSkill, Skill};

#[derive(Deserialize, SerdeDiff, Serialize, Debug, PartialEq, Clone, Copy)]
#[serde(transparent)]
/// A plant::Conf points to a plant::Config in the CONFIG lazy_static.
pub struct Conf(pub(crate) usize);

impl std::ops::Deref for Conf {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        config::CONFIG
            .plants
            .get(self.0)
            .as_ref()
            .expect("invalid plant Conf, this is very bad")
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub name: String,
    pub skillpoint_unlock_xps: Vec<usize>,
    #[serde(default)]
    pub base_yield_duration: Option<f32>,
    #[serde(default = "default_skills")]
    pub skills: config::FromFile<Vec<RawSkill>>,
}
fn default_skills() -> config::FromFile<Vec<RawSkill>> {
    config::FromFile::new(
        Default::default(),
        "please supply a skills file".to_string(),
    )
}

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub base_yield_duration: Option<f32>,
    pub skillpoint_unlock_xps: Vec<usize>,
    pub skills: Vec<Skill>,
}

impl config::Verify for RawConfig {
    type Verified = Config;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use config::FromFile;

        let corpus = ngrammatic::CorpusBuilder::new()
            .fill(self.skills.iter().map(|s| s.title.as_ref()))
            .finish();
        let skills_ref = self.skills.clone();
        let FromFile {
            inner: skills,
            file,
        } = self.skills;

        Ok(Config {
            name: self.name,
            base_yield_duration: self.base_yield_duration,
            skillpoint_unlock_xps: self.skillpoint_unlock_xps,
            skills: skills
                .into_iter()
                .map(|rsk| {
                    FromFile::new((skills_ref.as_slice(), &corpus, rsk), file.clone()).verify(raw)
                })
                .collect::<Result<_, _>>()?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in a plant named {}", self.name))
    }
}

#[derive(Clone, Debug, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Plant {
    pub owner_id: SteaderId,
    pub tile_id: TileId,
    pub xp: usize,
    pub nickname: String,
    pub conf: Conf,
    /// Records how many items have been applied to this plant
    /// over its lifetime (including effects that wore off long ago)
    pub lifetime_rubs: usize,
    pub craft: Option<Craft>,
    /// Effects from potions, warp powder, etc. that actively change the behavior of this plant.
    pub effects: Vec<Effect>,
}
impl Plant {
    pub fn from_conf(iu: impl IdentifiesSteader, it: impl IdentifiesTile, conf: Conf) -> Self {
        Self {
            owner_id: iu.steader_id(),
            tile_id: it.tile_id(),
            xp: 0,
            nickname: conf.name.clone(),
            conf,
            lifetime_rubs: 0,
            craft: None,
            effects: vec![],
        }
    }

    pub fn effect(&self, effect_id: EffectId) -> NoSuchResult<&Effect> {
        let &Self {
            owner_id,
            tile_id,
            ref effects,
            ..
        } = self;
        Ok(effects
            .iter()
            .find(|e| e.effect_id == effect_id)
            .ok_or_else(|| NoSuchEffectOnPlant(owner_id, tile_id, effect_id))?)
    }

    pub fn effect_mut(&mut self, effect_id: EffectId) -> NoSuchResult<&mut Effect> {
        let &mut Self {
            owner_id,
            tile_id,
            ref mut effects,
            ..
        } = self;
        Ok(effects
            .iter_mut()
            .find(|e| e.effect_id == effect_id)
            .ok_or_else(|| NoSuchEffectOnPlant(owner_id, tile_id, effect_id))?)
    }

    pub fn take_effect(&mut self, effect_id: EffectId) -> NoSuchResult<Effect> {
        let &mut Self {
            owner_id,
            tile_id,
            ref mut effects,
            ..
        } = self;
        Ok(effects
            .iter()
            .position(|e| e.effect_id == effect_id)
            .map(|i| self.effects.swap_remove(i))
            .ok_or_else(|| NoSuchEffectOnPlant(owner_id, tile_id, effect_id))?)
    }
}

pub mod timer {
    use super::*;

    #[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
    pub enum TimerKind {
        Yield,
        Craft { recipe_index: usize },
        Rub { effect_id: EffectId },
    }

    #[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
    pub enum Lifecycle {
        // when this timer finishes, it restarts again.
        Perennial { duration: f32 },
        // this timer runs once, then, kaputt.
        Annual,
    }

    impl Lifecycle {
        pub fn is_perennial(&self) -> bool {
            matches!(self, Lifecycle::Perennial { .. } )
        }

        pub fn is_annual(&self) -> bool {
            matches!(self, Lifecycle::Annual)
        }
    }

    #[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
    pub struct Timer {
        pub until_finish: f32,
        pub lifecycle: Lifecycle,
        pub tile_id: TileId,
        pub kind: TimerKind,
    }
}
pub use timer::{Timer, TimerKind};

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Craft {
    #[serde(alias = "makes")]
    pub recipe_archetype_handle: usize,
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
#[serde_diff(opaque)]
pub struct EffectId(pub uuid::Uuid);

impl fmt::Display for EffectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub enum EffectOrigin {
    Passive,
    Rub,
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    /// Records whether this is the first, second, third, etc. effect to be rubbed onto this plant.
    pub effect_id: EffectId,
    /// The conf of the item that was consumed to apply this effect.
    pub item_conf: item::Conf,
    /// The handle of the effect within this item that describes this effect.
    pub effect_archetype_handle: usize,
    /// How does an item give off this effect?
    pub origin: EffectOrigin,
}
impl std::ops::Deref for Effect {
    type Target = EffectConfig;

    fn deref(&self) -> &Self::Target {
        match self.origin {
            EffectOrigin::Rub => self
                .item_conf
                .plant_rub_effects
                .get(self.effect_archetype_handle)
                .as_ref()
                .expect("invalid rub effect_archetype_handle, this is pretty bad"),
            EffectOrigin::Passive => self
                .item_conf
                .passive_plant_effects
                .get(self.effect_archetype_handle)
                .as_ref()
                .expect("invalid passive effect_archetype_handle, this is pretty bad"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RawFilter {
    Only(Vec<String>),
    Not(Vec<String>),
    All,
}
impl Default for RawFilter {
    fn default() -> Self {
        RawFilter::All
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    Only(Vec<Conf>),
    Not(Vec<Conf>),
    All,
}
impl Default for Filter {
    fn default() -> Self {
        Filter::All
    }
}

impl config::Verify for RawFilter {
    type Verified = Filter;
    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let ok_or = |these: &[String]| {
            these
                .iter()
                .map(|p| raw.plant_conf(p))
                .collect::<Result<_, _>>()
        };

        Ok(match &self {
            RawFilter::Only(these) => Filter::Only(ok_or(these)?),
            RawFilter::Not(these) => Filter::Not(ok_or(these)?),
            RawFilter::All => Filter::All,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!(
            "in a {} filter",
            match self {
                RawFilter::Only(_) => "only",
                RawFilter::Not(_) => "not",
                RawFilter::All => "all",
            }
        ))
    }
}

impl Filter {
    pub fn allows(&self, c: Conf) -> bool {
        use Filter::*;

        match self {
            Only(these) => these.iter().any(|h| *h == c),
            Not(these) => !these.iter().any(|h| *h == c),
            All => true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawRecipe {
    pub title: String,
    /// This needs to get verified into an item::Conf pointing to an item to use the art of
    pub art: String,
    pub explanation: String,
    #[serde(default)]
    pub destroys_plant: bool,
    pub time: f32,
    /// Those Strings need to be verified into an item::Conf
    pub needs: Vec<(usize, String)>,
    /// This RawEvalput needs to be verified into an Evalput<item::Conf>
    pub makes: config::RawEvalput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub title: String,
    pub art: item::Conf,
    pub explanation: String,
    pub destroys_plant: bool,
    pub time: f32,
    pub needs: Vec<(usize, item::Conf)>,
    pub makes: config::Evalput<item::Conf>,
}

impl config::Verify for RawRecipe {
    type Verified = Recipe;
    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use config::VerifNote;

        Ok(Recipe {
            needs: self
                .needs
                .iter()
                .map(|(n, item_name)| Ok((*n, raw.item_conf(item_name)?)))
                .collect::<config::VerifResult<_>>()
                .note("in what the recipe needs")?,
            title: self.title,
            art: raw.item_conf(&self.art).note("in the art field")?,
            explanation: self.explanation,
            destroys_plant: self.destroys_plant,
            time: self.time,
            makes: self.makes.verify(raw).note("in what the recipe makes")?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in a recipe named \"{}\"", self.title))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawEffectConfig {
    pub description: String,
    pub buff: Option<RawBuff>,
    #[serde(default)]
    pub for_plants: RawFilter,
    #[serde(default)]
    pub duration: Option<f32>,
    #[serde(default)]
    pub transmogrification: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectConfig {
    pub description: String,
    pub kind: EffectConfigKind,
    pub for_plants: Filter,
    pub duration: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectConfigKind {
    Buff(Buff),
    Transmogrification(Conf),
}

impl config::Verify for RawEffectConfig {
    type Verified = EffectConfig;
    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use config::VerifNote;

        let transmogrification = self
            .transmogrification
            .as_ref()
            .map(|plant_name| raw.plant_conf(plant_name))
            .transpose()
            .note("in the transmogrification field")?;
        let buff = self.buff.clone().verify(raw)?;

        Ok(EffectConfig {
            kind: match (buff, transmogrification) {
                (Some(buff), None) => Ok(EffectConfigKind::Buff(buff)),
                (None, Some(trans)) => Ok(EffectConfigKind::Transmogrification(trans)),
                (Some(_), Some(_)) => Err(config::VerifError::custom(
                    "a single effect should not transmog AND buff",
                )),
                (None, None) => Err(config::VerifError::custom(
                    "an effect should either transmog OR buff",
                )),
            }?,
            description: self.description,
            for_plants: self.for_plants.verify(raw)?,
            duration: self.duration,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!(
            "in an effect described \"{}...\"",
            &self.description[0..20.min(self.description.len())]
        ))
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{ClientError, ClientResult},
        wormhole::{ask, until_ask_id_map, AskedNote, PlantAsk},
        Ask, IdentifiesItem,
    };

    impl Plant {
        pub async fn slaughter(&self) -> ClientResult<Plant> {
            let a = Ask::Plant(PlantAsk::Slaughter {
                tile_id: self.tile_id,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantSlaughterResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantSlaughter", e))
        }

        pub async fn rub_with(&self, rub: impl IdentifiesItem) -> ClientResult<Vec<Effect>> {
            let a = Ask::Plant(PlantAsk::Rub {
                rub_item_id: rub.item_id(),
                tile_id: self.tile_id,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantRubStartResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantRub", e))
        }
    }
}
