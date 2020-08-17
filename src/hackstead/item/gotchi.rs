use crate::id::{IdentifiesItem, ItemId};
use crate::{config, CONFIG};
use config::ArchetypeHandle;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Gotchi {
    pub nickname: String,
    pub item_id: ItemId,
}
impl Gotchi {
    pub fn new(ah: ArchetypeHandle, ii: impl IdentifiesItem) -> Self {
        Self {
            nickname: CONFIG.item(ah).unwrap().name.clone(),
            item_id: ii.item_id(),
        }
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{ClientError, ClientResult},
        wormhole::{ask, until_ask_id_map, AskedNote, ItemAsk},
        Ask, IdentifiesItem,
    };

    impl Gotchi {
        pub async fn rename(&mut self, new_name: String) -> ClientResult<String> {
            let a = Ask::Item(ItemAsk::GotchiRename {
                item_id: self.item_id,
                new_name,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::GotchiRenameResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "GotchiRename", e))
        }
    }
}
