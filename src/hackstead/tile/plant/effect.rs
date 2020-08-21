use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;

use super::{Buff, Conf, Filter};
#[cfg(feature = "config_verify")]
use super::{RawBuff, RawFilter};
use crate::{config, item};

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
pub enum Origin {
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
    pub origin: Origin,
}

impl std::ops::Deref for Effect {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        match self.origin {
            Origin::Rub => self
                .item_conf
                .plant_rub_effects
                .get(self.effect_archetype_handle)
                .as_ref()
                .expect("invalid rub effect_archetype_handle, this is pretty bad"),
            Origin::Passive => self
                .item_conf
                .passive_plant_effects
                .get(self.effect_archetype_handle)
                .as_ref()
                .expect("invalid passive effect_archetype_handle, this is pretty bad"),
        }
    }
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    pub description: String,
    pub buff: Option<RawBuff>,
    #[serde(default)]
    pub for_plants: RawFilter,
    #[serde(default)]
    pub duration: Option<f32>,
    #[serde(default)]
    pub transmogrification: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub description: String,
    pub kind: ConfigKind,
    pub for_plants: Filter,
    pub duration: Option<f32>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum ConfigKind {
    Buff(Buff),
    Transmogrification(Conf),
}

#[cfg(feature = "config_verify")]
impl config::Verify for RawConfig {
    type Verified = Config;
    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use config::VerifNote;

        let transmogrification = self
            .transmogrification
            .as_ref()
            .map(|plant_name| raw.plant_conf(plant_name))
            .transpose()
            .note("in the transmogrification field")?;
        let buff = self.buff.clone().verify(raw)?;

        Ok(Config {
            kind: match (buff, transmogrification) {
                (Some(buff), None) => Ok(ConfigKind::Buff(buff)),
                (None, Some(trans)) => Ok(ConfigKind::Transmogrification(trans)),
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
