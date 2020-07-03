#[cfg(feature = "config_verify")]
use crate::config::{self, Verify};
use crate::item;
#[cfg(feature = "config_verify")]
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
#[cfg(feature = "config_verify")]
use std::fmt;

#[derive(Deserialize, serde_diff::SerdeDiff, Serialize, Debug, PartialEq, Clone, Copy)]
#[serde_diff(opaque)]
/// A skill::Conf points to an skill::Skill on a certain plant's list of Skills.
/// also contains the Conf of the plant.
pub struct Conf(pub(crate) super::Conf, pub(crate) uuid::Uuid);

impl std::ops::Deref for Conf {
    type Target = Skill;

    #[cfg(feature = "config_verify")]
    fn deref(&self) -> &Self::Target {
        panic!("no dereffing confs while verifying config")
    }

    #[cfg(not(feature = "config_verify"))]
    fn deref(&self) -> &Self::Target {
        self.0
            .skills
            .get(&self.1)
            .as_ref()
            .expect("invalid skill Conf, this is very bad")
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RawCost {
    hide_until_met: bool,
    points: usize,
    items: Vec<(usize, String)>,
    skills: Vec<String>,
}

#[cfg(feature = "config_verify")]
impl Default for RawCost {
    fn default() -> Self {
        RawCost {
            hide_until_met: false,
            points: 1,
            items: vec![],
            skills: vec![],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Cost {
    hide_until_met: bool,
    points: usize,
    items: Vec<(usize, item::Conf)>,
    skills: Vec<Conf>,
}

#[cfg(feature = "config_verify")]
impl Verify for (super::Conf, RawCost) {
    type Verified = Cost;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (plant_conf, cost) = self;
        Ok(Cost {
            hide_until_met: cost.hide_until_met,
            points: cost.points,
            skills: cost
                .skills
                .iter()
                .map(|s| raw.plant_skill_conf(plant_conf, s))
                .collect::<Result<_, _>>()?,
            items: cost
                .items
                .iter()
                .map(|(n, i)| Ok((*n, raw.item_conf(i)?)))
                .collect::<Result<_, _>>()?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in one of an unlock's costs"))
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawUnlock {
    skill: String,
    costs: Vec<RawCost>,
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub struct SkillNameOrRawUnlock(#[serde(deserialize_with = "skill_name_or_raw_unlock")] RawUnlock);
#[cfg(feature = "config_verify")]
fn skill_name_or_raw_unlock<'de, D>(deserializer: D) -> Result<RawUnlock, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use de::value::MapAccessDeserializer;

    struct Deser;
    impl<'de> Visitor<'de> for Deser {
        type Value = RawUnlock;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "`string (skill name)` OR `map`")
        }

        fn visit_str<E>(self, s: &str) -> Result<RawUnlock, E>
        where
            E: de::Error,
        {
            Ok(RawUnlock {
                skill: s.to_string(),
                costs: vec![Default::default()],
            })
        }

        fn visit_map<M>(self, map: M) -> Result<RawUnlock, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_any(Deser)
}

#[cfg(feature = "config_verify")]
impl std::ops::Deref for SkillNameOrRawUnlock {
    type Target = RawUnlock;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Unlock {
    skill: Conf,
    costs: Vec<Cost>,
}

#[cfg(feature = "config_verify")]
impl Verify for (super::Conf, SkillNameOrRawUnlock) {
    type Verified = Unlock;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (plant_conf, SkillNameOrRawUnlock(RawUnlock { skill, costs })) = self;
        Ok(Unlock {
            skill: raw.plant_skill_conf(plant_conf, &skill)?,
            costs: costs
                .into_iter()
                .map(|c| (plant_conf, c))
                .collect::<Vec<_>>()
                .verify(raw)?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in an unlock for {}", self.1.skill))
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawSkill {
    pub title: String,
    pub unlocks: Vec<SkillNameOrRawUnlock>,
    pub effects: Vec<super::effect::RawConfig>,
    pub conf: uuid::Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Skill {
    pub title: String,
    pub unlocks: Vec<Unlock>,
    pub effects: Vec<super::effect::Config>,
}
#[cfg(feature = "config_verify")]
impl Verify for (super::Conf, RawSkill) {
    type Verified = Skill;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (plant_conf, rsk) = self;
        let unlocks = rsk
            .unlocks
            .into_iter()
            .map(|unlock| (plant_conf, unlock).verify(raw))
            .collect::<Result<_, _>>()?;

        Ok(Skill {
            unlocks,
            title: rsk.title,
            effects: rsk.effects.verify(raw)?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in a skill titled {}", self.1.title))
    }
}
