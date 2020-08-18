use super::{RawRecipe, Recipe};
use crate::{config, item};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, serde_diff::SerdeDiff, Serialize, Debug, PartialEq, Clone, Copy)]
#[serde(transparent)]
/// A skill::Conf points to an skill::Skill on a certain plant's list of Skills
pub struct Conf(pub(crate) usize);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RawBuffKind {
    Neighbor(Box<RawBuffKind>),
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuffKind {
    Neighbor(Box<BuffKind>),
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
}
impl config::Verify for RawBuffKind {
    type Verified = BuffKind;
    fn verify(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use BuffKind as B;
        use RawBuffKind::*;
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
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawBuff {
    pub kind: RawBuffKind,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Buff {
    pub kind: BuffKind,
    pub title: String,
    pub description: String,
}

impl config::Verify for RawBuff {
    type Verified = Buff;
    fn verify(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        Ok(Buff {
            kind: self.kind.verify(raw)?,
            title: self.title,
            description: self.description,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawSkill {
    unlocks: Vec<String>,
    buff: RawBuff,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    unlocks: Vec<Conf>,
    buff: Buff,
}
impl config::Verify for (&[RawSkill], RawSkill) {
    type Verified = Skill;
    fn verify(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (skills, rsk) = self;
        Ok(Skill {
            unlocks: rsk
                .unlocks
                .iter()
                .map(
                    |skill_title| match skills.iter().position(|s| s.buff.title == *skill_title) {
                        None => Err(config::VerifError::Custom(format!(
                            "no such skill with title {}",
                            skill_title
                        ))),
                        Some(i) => Ok(Conf(i)),
                    },
                )
                .collect::<Result<_, _>>()?,
            buff: rsk.buff.verify(raw)?,
        })
    }
}
