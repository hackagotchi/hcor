use crate::{config, item};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(Deserialize, SerdeDiff, Serialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
#[serde_diff(opaque)]
/// A plant::Conf points to a plant::Config in the CONFIG lazy_static.
pub struct Conf(pub(crate) uuid::Uuid);

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Craft {
    #[serde(alias = "makes")]
    pub recipe_conf: Conf,
}

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawRecipe {
    pub title: String,
    pub conf: Conf,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Recipe {
    pub title: String,
    pub conf: Conf,
    pub art: item::Conf,
    pub explanation: String,
    pub destroys_plant: bool,
    pub time: f32,
    pub needs: Vec<(usize, item::Conf)>,
    pub makes: config::Evalput<item::Conf>,
}

#[cfg(feature = "config_verify")]
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
            conf: self.conf,
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
