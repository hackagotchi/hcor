use crate::{config, CONFIG};
use config::ArchetypeHandle;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Gotchi {
    pub nickname: String,
}
impl Gotchi {
    pub fn new(ah: ArchetypeHandle) -> Self {
        Self {
            nickname: CONFIG.item(ah).unwrap().name.clone(),
        }
    }
}
