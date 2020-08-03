use crate::{config, CONFIG};
use config::ArchetypeHandle;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Gotchi {
    pub item_id: uuid::Uuid,
    pub nickname: String,
}
impl Gotchi {
    pub fn new(item_id: uuid::Uuid, ah: ArchetypeHandle) -> Self {
        Self {
            item_id,
            nickname: CONFIG.item(ah)
                .unwrap()
                .name
                .clone(),
        }
    }
}
