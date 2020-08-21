#[cfg(feature = "config_verify")]
use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, serde_diff::SerdeDiff, Serialize, Debug, PartialEq, Clone, Copy)]
/// A skill::Conf points to an skill::Skill on a certain plant's list of Skills.
/// also contains the Conf of the plant.
pub struct Conf(pub(crate) super::Conf, pub(crate) usize);

impl std::ops::Deref for Conf {
    type Target = Skill;

    fn deref(&self) -> &Self::Target {
        self.0
            .skills
            .get(self.1)
            .as_ref()
            .expect("invalid skill Conf, this is very bad")
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawSkill {
    pub title: String,
    pub unlocks: Vec<String>,
    pub effects: Vec<super::effect::RawConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Skill {
    pub title: String,
    pub unlocks: Vec<Conf>,
    pub effects: Vec<super::effect::Config>,
}
#[cfg(feature = "config_verify")]
impl config::Verify for (super::Conf, &[RawSkill], &ngrammatic::Corpus, RawSkill) {
    type Verified = Skill;

    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        let (plant_conf, skills, corpus, rsk) = self;
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
                    Some(i) => Ok(Conf(plant_conf, i)),
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
        Some(format!("in a skill titled {}", self.3.title.clone()))
    }
}
