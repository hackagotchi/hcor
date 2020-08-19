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
        precedence: usize,
    },
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
    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
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
            Art { file, precedence } => B::Art { file, precedence },
        })
    }

    fn context(&self) -> Option<String> {
        use RawBuff::*;
        Some(format!(
            "in a{} buff",
            match self {
                Neighbor(_) => " neighbor",
                ExtraTimeTicks(_) => "n extra time ticks",
                TimeTicksMultiplier(_) => " time ticks multiplier",
                Xp(_) => " xp",
                YieldSpeedMultiplier(_) => " yield speed multiplier",
                YieldSizeMultiplier(_) => " yield size multipler",
                Yield(_) => " yield",
                Craft(_) => " craft",
                CraftSpeedMultiplier(_) => " craft speed multiplier",
                CraftReturnChance(_) => " craft return chance",
                DoubleCraftYield(_) => " double craft yield",
                Art { .. } => "n art",
            }
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawSkill {
    pub title: String,
    pub unlocks: Vec<String>,
    pub effects: Vec<super::RawEffectConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    pub title: String,
    pub unlocks: Vec<Conf>,
    pub effects: Vec<super::EffectConfig>,
}
impl config::Verify for (&[RawSkill], &ngrammatic::Corpus, RawSkill) {
    type Verified = Skill;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (skills, corpus, rsk) = self;
        let unlocks = rsk
            .unlocks
            .iter()
            .map(
                |skill_title| match skills.iter().position(|s| s.title == *skill_title) {
                    None => Err(config::VerifError::custom(format!(
                        "referenced skill titled {}, \
                            but no skill with this title could be found. \
                            Perhaps you meant {}?",
                        skill_title,
                        corpus
                            .search(skill_title, 0.2)
                            .into_iter()
                            .map(|s| s.text)
                            .collect::<Vec<_>>()
                            .join(", or "),
                    ))),
                    Some(i) => Ok(Conf(i)),
                },
            )
            .collect::<Result<_, _>>()?;

        Ok(Skill {
            unlocks,
            title: rsk.title,
            effects: rsk.effects.verify(raw)?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in a skill titled {}", self.2.title.clone()))
    }
}
