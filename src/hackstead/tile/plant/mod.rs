use crate::{
    config,
    id::{NoSuchResult, NoSuchRubEffectOnPlant},
    Hackstead, IdentifiesSteader, IdentifiesTile, SteaderId, TileId,
};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;

pub mod skill;
#[cfg(feature = "config_verify")]
pub use skill::RawSkill;
pub use skill::Skill;

pub mod recipe;
#[cfg(feature = "config_verify")]
pub use recipe::RawRecipe;
pub use recipe::{Craft, Recipe};

pub mod buff;
#[cfg(feature = "config_verify")]
pub use buff::RawBuff;
pub use buff::{Buff, BuffBook, BuffSum};

pub mod effect;
pub use effect::{RubEffect, RubEffectId};

pub mod timer;
pub use timer::{Timer, TimerKind};

#[derive(Deserialize, SerdeDiff, Serialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
#[serde_diff(opaque)]
/// A plant::Conf points to a plant::Config in the CONFIG lazy_static.
pub struct Conf(pub(crate) uuid::Uuid);

impl Conf {
    pub fn try_lookup(self) -> Option<&'static Config> {
        config::CONFIG.plants.get(&self)
    }
}

impl std::ops::Deref for Conf {
    type Target = Config;

    #[cfg(feature = "config_verify")]
    fn deref(&self) -> &Self::Target {
        panic!("no looking up confs with config_verify enabled")
    }

    #[cfg(not(feature = "config_verify"))]
    fn deref(&self) -> &Self::Target {
        self.try_lookup()
            .expect("invalid plant Conf, this is very bad")
    }
}

impl fmt::Display for Conf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub name: String,
    pub conf: Conf,
    pub skillpoint_unlock_xps: Vec<usize>,
    #[serde(default)]
    pub base_yield_duration: Option<f32>,
    #[serde(default = "default_skills")]
    pub skills: config::FromFile<Vec<RawSkill>>,
}
#[cfg(feature = "config_verify")]
fn default_skills() -> config::FromFile<Vec<RawSkill>> {
    config::FromFile::new(
        Default::default(),
        "please supply a skills file".to_string(),
    )
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub name: String,
    pub conf: Conf,
    pub base_yield_duration: Option<f32>,
    pub skillpoint_unlock_xps: Vec<usize>,
    pub skills: config::ConfMap<uuid::Uuid, Skill>,
}

#[cfg(feature = "config_verify")]
impl config::Verify for RawConfig {
    type Verified = Config;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let plant_conf = self.conf;
        Ok(Config {
            name: self.name,
            conf: plant_conf,
            base_yield_duration: self.base_yield_duration,
            skillpoint_unlock_xps: self.skillpoint_unlock_xps,
            skills: self
                .skills
                .map(|s| {
                    s.into_iter()
                        .map(|rsk| (rsk.conf, (plant_conf, rsk)))
                        .collect::<config::ConfMap<uuid::Uuid, _>>()
                })
                .verify(raw)?,
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
    pub nickname: String,
    pub conf: Conf,
    /// Records how many items have been applied to this plant
    /// over its lifetime (including effects that wore off long ago)
    pub lifetime_rubs: usize,
    pub craft: Option<Craft>,
    /// Effects from potions, warp powder, etc. that actively change the behavior of this plant.
    pub rub_effects: Vec<RubEffect>,
    pub skills: Skills,
}

impl std::ops::Deref for Plant {
    type Target = Config;

    #[cfg(feature = "config_verify")]
    fn deref(&self) -> &Self::Target {
        panic!("no looking up confs with config_verify enabled")
    }

    #[cfg(not(feature = "config_verify"))]
    fn deref(&self) -> &Self::Target {
        &*self.conf
    }
}

#[derive(Clone, Debug, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Skills {
    points_redeemed: usize,
    conf: Conf,
    level: usize,
    xp: usize,
    unlocked: Vec<skill::Conf>,
}

impl Skills {
    pub fn empty(conf: Conf) -> Self {
        Self {
            points_redeemed: 0,
            conf,
            level: 0,
            xp: 0,
            unlocked: vec![],
        }
    }

    fn point_xps(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.conf.skillpoint_unlock_xps.iter().copied()
    }

    pub fn next_point_info(&self) -> config::LevelInfo {
        config::max_level_info(self.xp, self.point_xps())
    }

    pub fn available_points(&self) -> usize {
        let awarded = config::max_level_index(self.xp, self.point_xps());
        awarded - self.points_redeemed
    }

    /// If can't afford this amount of skill points, returns the amount that would be needed.
    pub fn afford(&self, amount: usize) -> Result<(), usize> {
        if self.available_points() >= amount {
            Ok(())
        } else {
            Err(amount - self.available_points())
        }
    }

    /// If can't afford this amount of skill points, returns the amount that would be needed.
    pub fn charge(&mut self, amount: usize) -> Result<(), usize> {
        self.afford(amount).map(|_| self.points_redeemed += amount)
    }
}

impl Plant {
    pub fn from_conf(iu: impl IdentifiesSteader, it: impl IdentifiesTile, conf: Conf) -> Self {
        Self {
            owner_id: iu.steader_id(),
            tile_id: it.tile_id(),
            nickname: conf.name.clone(),
            conf,
            lifetime_rubs: 0,
            craft: None,
            rub_effects: vec![],
            skills: Skills::empty(conf),
        }
    }

    /// includes:
    ///  - passive item buffs
    ///  - rub effects
    ///  - unlocked skills
    ///
    /// does NOT include:
    ///  - neighbor bonuses (any of the above, but from other plants)
    ///
    /// If you want neighbor bonuses, consult your local BuffBook.
    fn buffs(&self, hs: &Hackstead) -> Vec<(Buff, buff::Source)> {
        use buff::Source::*;

        // buffs from passive items
        hs.inventory
            .iter()
            .map(|i| &i.conf)
            .flat_map(|ic| {
                ic.passive_plant_effects
                    .iter()
                    .filter(|e| e.for_plants.allows(self.conf))
                    .filter_map(move |e| Some((e.kind.buff()?.clone(), PassiveItemEffect(*ic))))
            })
            // buffs from rub effects
            .chain(
                self.rub_effects
                    .iter()
                    .filter_map(|e| Some((e.kind.buff()?.clone(), RubbedItemEffect(e.item_conf)))),
            )
            // buffs from unlocked skills
            .chain(self.skills.unlocked.iter().flat_map(|sc| {
                sc.effects
                    .iter()
                    .filter_map(move |e| Some((e.kind.buff()?.clone(), SkillUnlock(*sc))))
            }))
            // boom, one big fat allocation
            .collect()
    }

    pub fn rub_effect(&self, effect_id: RubEffectId) -> NoSuchResult<&RubEffect> {
        let &Self {
            owner_id,
            tile_id,
            ref rub_effects,
            ..
        } = self;
        Ok(rub_effects
            .iter()
            .find(|e| e.effect_id == effect_id)
            .ok_or_else(|| NoSuchRubEffectOnPlant(owner_id, tile_id, effect_id))?)
    }

    pub fn rub_effect_mut(&mut self, effect_id: RubEffectId) -> NoSuchResult<&mut RubEffect> {
        let &mut Self {
            owner_id,
            tile_id,
            ref mut rub_effects,
            ..
        } = self;
        Ok(rub_effects
            .iter_mut()
            .find(|e| e.effect_id == effect_id)
            .ok_or_else(|| NoSuchRubEffectOnPlant(owner_id, tile_id, effect_id))?)
    }

    pub fn take_rub_effect(&mut self, effect_id: RubEffectId) -> NoSuchResult<RubEffect> {
        let &mut Self {
            owner_id,
            tile_id,
            ref mut rub_effects,
            ..
        } = self;
        Ok(rub_effects
            .iter()
            .position(|e| e.effect_id == effect_id)
            .map(|i| rub_effects.swap_remove(i))
            .ok_or_else(|| NoSuchRubEffectOnPlant(owner_id, tile_id, effect_id))?)
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RawFilter {
    Only(Vec<String>),
    Not(Vec<String>),
    All,
}
#[cfg(feature = "config_verify")]
impl Default for RawFilter {
    fn default() -> Self {
        RawFilter::All
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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

#[cfg(feature = "config_verify")]
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

        pub async fn rub_with(&self, rub: impl IdentifiesItem) -> ClientResult<Vec<RubEffect>> {
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
