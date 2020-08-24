use crate::id::{IdentifiesItem, ItemId};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    base_happiness: usize,
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Gotchi {
    pub nickname: String,
    pub item_id: ItemId,
}
impl Gotchi {
    pub fn new(conf: super::Conf, ii: impl IdentifiesItem) -> Self {
        Self {
            nickname: conf.name.clone(),
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
        Ask,
    };

    impl Gotchi {
        pub async fn rename(&self, new_name: String) -> ClientResult<String> {
            let a = Ask::Item(ItemAsk::GotchiNickname {
                item_id: self.item_id,
                new_name,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::GotchiNicknameResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "GotchiNickname", e))
        }
    }
}
