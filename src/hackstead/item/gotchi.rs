use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    base_happiness: usize,
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Gotchi {
    pub nickname: String,
}
impl Gotchi {
    pub fn new(conf: super::Conf) -> Self {
        Self {
            nickname: conf.name.clone(),
        }
    }
}
