use super::{RawRecipe, Recipe};
use crate::{config, item};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, serde_diff::SerdeDiff, Serialize, Debug, PartialEq, Clone, Copy)]
#[serde(transparent)]
/// A skill::Conf points to an skill::Skill on a certain plant's list of Skills
pub struct Conf(pub(crate) usize);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RawBuff {
    Neighbor(Box<RawBuff>),
    ExtraTimeTicks(usize),
    TimeTicksMultiplier(f32),
    Xp(f32),
    YieldSpeedMultiplier(f32),
    YieldSizeMultiplier(f32),
    /// This RawEvalput needs to be verified into an Evalput<item::Conf>
    Yield(config::RawEvalput),
    /// This RawRecipe needs to be verified into a Recipe
    Craft(Vec<RawRecipe>),
    CraftSpeedMultiplier(f32),
    CraftReturnChance(f32),
    DoubleCraftYield(f32),
    Art {
        file: String,
        precedence: usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Buff {
    Neighbor(Box<Buff>),
    /// Stores the number of extra cycles to add for the duration of the effect
    ExtraTimeTicks(usize),
    TimeTicksMultiplier(f32),
    Xp(f32),
    YieldSpeedMultiplier(f32),
    YieldSizeMultiplier(f32),
    Yield(config::Evalput<item::Conf>),
    Craft(Vec<Recipe>),
    CraftSpeedMultiplier(f32),
    CraftReturnChance(f32),
    DoubleCraftYield(f32),
    Art {
        file: String,
        precedence: usize,
    },
}
impl config::Verify for RawBuff {
    type Verified = Buff;
    fn verify(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use Buff as B;
        use RawBuff::*;
        Ok(match self {
            Neighbor(b) => B::Neighbor(Box::new(b.verify(raw)?)),
            ExtraTimeTicks(u) => B::ExtraTimeTicks(u),
            TimeTicksMultiplier(f) => B::TimeTicksMultiplier(f),
            Xp(f) => B::Xp(f),
            YieldSpeedMultiplier(f) => B::YieldSpeedMultiplier(f),
            YieldSizeMultiplier(f) => B::YieldSizeMultiplier(f),
            Yield(evalput) => B::Yield(evalput.verify(raw)?),
            Craft(recipes) => B::Craft(recipes.verify(raw)?),
            CraftSpeedMultiplier(f) => B::CraftSpeedMultiplier(f),
            CraftReturnChance(f) => B::CraftReturnChance(f),
            DoubleCraftYield(f) => B::DoubleCraftYield(f),
            Art { file, precedence } => B::Art { file, precedence }
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawSkill {
    title: String,
    unlocks: Vec<String>,
    effects: Vec<super::RawEffectConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    title: String,
    unlocks: Vec<Conf>,
    effects: Vec<super::EffectConfig>,
}
impl config::Verify for (&[RawSkill], RawSkill) {
    type Verified = Skill;
    fn verify(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (skills, rsk) = self;
        Ok(Skill {
            title: rsk.title,
            unlocks: rsk
                .unlocks
                .iter()
                .map(
                    |skill_title| match skills.iter().position(|s| s.title == *skill_title) {
                        None => Err(config::VerifError::Custom(format!(
                            "no such skill with title {}",
                            skill_title
                        ))),
                        Some(i) => Ok(Conf(i)),
                    },
                )
                .collect::<Result<_, _>>()?,
            effects: rsk.effects.verify(raw)?,
        })
    }
}
