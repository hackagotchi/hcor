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

impl Conf {
    pub fn try_lookup(self) -> Option<&'static Skill> {
        self.0.try_lookup()?.skills.get(&self.1)
    }
}

impl std::ops::Deref for Conf {
    type Target = Skill;

    #[cfg(feature = "config_verify")]
    fn deref(&self) -> &Self::Target {
        panic!("no dereffing confs while verifying config")
    }

    #[cfg(not(feature = "config_verify"))]
    fn deref(&self) -> &Self::Target {
        self.try_lookup()
            .expect("invalid skill Conf, this is very bad")
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct RawCost {
    points: usize,
    items: Vec<(usize, String)>,
    skills: Vec<String>,
}

#[cfg(feature = "config_verify")]
impl RawCost {
    pub fn empty() -> Self {
        Default::default()
    }

    pub fn points(points: usize) -> Self {
        RawCost {
            points,
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Cost {
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
    costs: RawCost,
    #[serde(default)]
    hide_until: RawCost,
}

#[cfg(feature = "config_verify")]
struct RawUnlockVerifyWrapper {
    inner: RawUnlock,
    source_skill: Conf,
    plant_conf: super::Conf,
    index: usize,
}

#[cfg(feature = "config_verify")]
impl Verify for RawUnlockVerifyWrapper {
    type Verified = Unlock;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let RawUnlockVerifyWrapper {
            inner:
                RawUnlock {
                    skill,
                    hide_until,
                    costs,
                },
            source_skill,
            plant_conf,
            index,
        } = self;
        Ok(Unlock {
            skill: raw.plant_skill_conf(plant_conf, &skill)?,
            hide_until: (plant_conf, hide_until).verify(raw)?,
            costs: (plant_conf, costs).verify(raw)?,
            source_skill,
            index,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in an unlock for {}", self.inner.skill))
    }
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
                costs: RawCost::points(1),
                hide_until: RawCost::empty(),
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
    hide_until: Cost,
    costs: Cost,
    source_skill: Conf,
    index: usize,
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{ClientError, ClientResult},
        wormhole::{ask, until_ask_id_map, AskedNote, PlantAsk},
        Ask, IdentifiesPlant,
    };

    impl Unlock {
        pub async fn unlock_for(&self, p: impl IdentifiesPlant) -> ClientResult<usize> {
            let tile_id = p.tile_id();
            let a = Ask::Plant(PlantAsk::SkillUnlock {
                tile_id,
                source_skill_conf: self.source_skill,
                unlock_index: self.index,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::PlantSkillUnlockResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "PlantSkillUnlock", e))
        }
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
        let uuid = rsk.conf;

        Ok(Skill {
            title: rsk.title,
            effects: rsk.effects.verify(raw)?,
            unlocks: rsk
                .unlocks
                .into_iter()
                .enumerate()
                .map(|(index, SkillNameOrRawUnlock(unlock))| {
                    RawUnlockVerifyWrapper {
                        plant_conf,
                        index,
                        source_skill: Conf(plant_conf, uuid),
                        inner: unlock,
                    }
                    .verify(raw)
                })
                .collect::<Result<_, _>>()?,
        })
    }

    fn context(&self) -> Option<String> {
        Some(format!("in a skill titled {}", self.1.title))
    }
}
