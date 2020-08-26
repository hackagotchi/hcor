use crate::{config, item, plant, Item, ItemId, Plant, SteaderId, Tile, TileId};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{
    ask, connect, disconnect, register_note_handler, try_note, until, until_ask_id,
    until_ask_id_map, until_ask_id_map_greedy, until_greedy, until_map, WormholeError,
    WormholeResult,
};

/// How often heartbeat pings are sent
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
/// How long before lack of client response causes a timeout
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long before lack of server response causes a timeout
pub const SERVER_TIMEOUT: Duration = Duration::from_secs(25);

type StrResult<T> = Result<T, String>;

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
/// AskedNotes are immediate responses to things explicitly requested by the client using an
/// AskMessage.
///
/// Almost all of these Results have string error messages.
pub enum AskedNote {
    /// This event is actually infallible,
    ///
    /// Returns the new total xp of the user's stead.
    KnowledgeSnortResult(StrResult<usize>),

    /// Can fail if this tile is already occupied, among a host of other reasons
    ///
    /// Returns the fresh new plant, if successful.
    PlantSummonResult(StrResult<Plant>),

    /// Can fail if the plant doesn't exist, among a host of other reasons.
    ///
    /// Returns the now-deceased plant, if successful.
    PlantSlaughterResult(StrResult<Plant>),

    /// Expect a RudeNote::CraftFinish later.
    ///
    /// Can fail if the plant is already crafting something, among a host of other reasons.
    ///
    /// Returns the Craft struct added to the plant, if successful.
    PlantCraftStartResult(StrResult<plant::Craft>),

    /// Expect a RudeNote::RubEffectFinish later, if the item you applied can wear off.
    ///
    /// Can fail if the provided item has no rub effects or has no rub effects when applied to this
    /// plant, among a host of other reasons.
    ///
    /// Returns the effect struct, complete with ID and ticks until finish.
    PlantRubStartResult(StrResult<Vec<plant::RubEffect>>),

    /// Result of renaming a plant
    ///
    /// Returns the new name
    PlantNicknameResult(StrResult<String>),

    /// Can fail if you don't have any skill points to spare,
    /// or if there is no skill with the id you asked for.
    ///
    /// Returns the number of skill points left, if successful.
    PlantSkillUnlockResult(StrResult<usize>),

    /// Can fail if the plant doesn't exist.
    ///
    /// Returns the new xp total for this plant.
    PlantKnowledgeSnortResult(StrResult<usize>),

    /// Summoning a tile can fail if the item used isn't configured to do so.
    ///
    /// Returns the fresh tile, if successful.
    TileSummonResult(StrResult<Tile>),

    /// This can fail if an invalid item_conf is provided, or if the user is not authorized to
    /// spawn items.
    ///
    /// Returns the list of new items, if successful.
    ItemSpawnResult(StrResult<Vec<Item>>),

    /// This can fail if the items don't belong to the giver.
    ///
    /// Returns the list of new items, complete with updated owner logs.
    ItemThrowResult(StrResult<Vec<Item>>),

    /// This can fail if the provided item isn't hatchable, among a host of other reasons.
    ///
    /// Returns a list of the new items, if successful.
    ItemHatchResult(StrResult<config::evalput::Output<Item>>),

    /// The result of renaming a gotchi
    ///
    /// Returns the new name
    GotchiNicknameResult(StrResult<String>),
}

impl AskedNote {
    /// Returns an AskedNote's error message, if any
    pub fn err(&self) -> Option<&str> {
        use AskedNote::*;
        // I know this is cursed af, but I wanted to match exhaustively so that the compiler
        // would warn me if I didn't add a new entry.
        //
        // If this bothers you, PR in a macro to generate this automatically?
        match self {
            KnowledgeSnortResult(Err(e)) => Some(e),
            KnowledgeSnortResult(Ok(_)) => None,
            PlantSummonResult(Err(e)) => Some(e),
            PlantSummonResult(Ok(_)) => None,
            PlantSlaughterResult(Err(e)) => Some(e),
            PlantSlaughterResult(Ok(_)) => None,
            PlantCraftStartResult(Err(e)) => Some(e),
            PlantCraftStartResult(Ok(_)) => None,
            PlantRubStartResult(Err(e)) => Some(e),
            PlantRubStartResult(Ok(_)) => None,
            PlantNicknameResult(Err(e)) => Some(e),
            PlantNicknameResult(Ok(_)) => None,
            PlantSkillUnlockResult(Err(e)) => Some(e),
            PlantSkillUnlockResult(Ok(_)) => None,
            PlantKnowledgeSnortResult(Err(e)) => Some(e),
            PlantKnowledgeSnortResult(Ok(_)) => None,
            TileSummonResult(Err(e)) => Some(e),
            TileSummonResult(Ok(_)) => None,
            ItemSpawnResult(Err(e)) => Some(e),
            ItemSpawnResult(Ok(_)) => None,
            ItemThrowResult(Err(e)) => Some(e),
            ItemThrowResult(Ok(_)) => None,
            ItemHatchResult(Err(e)) => Some(e),
            ItemHatchResult(Ok(_)) => None,
            GotchiNicknameResult(Err(e)) => Some(e),
            GotchiNicknameResult(Ok(_)) => None,
        }
    }
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
/// Rude Notes aren't responses to any particular Ask from the client,
/// they sort of barge in unnanounced.
pub enum RudeNote {
    /// Identifies the giver and contains a list of items, complete with updated ownership logs.
    ItemThrowReceipt { from: SteaderId, items: Vec<Item> },
    YieldFinish {
        output: config::evalput::Output<Item>,
        tile_id: TileId,
    },
    CraftFinish {
        output: config::evalput::Output<Item>,
        tile_id: TileId,
    },
    RubEffectFinish {
        effect: plant::RubEffect,
        tile_id: TileId,
    },
    TimerUpdate {
        timer_id: plant::TimerId,
        value: f32,
        rate: f32,
    },
}

/// Like a notification, but cuter; a tidbit of information that the server
/// thinks you might have a special interest in.
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum Note {
    Rude(RudeNote),
    Edit(EditNote),
    Asked {
        note: AskedNote,
        /// The id is only stored on the server to be sent back with the AskedNote, and is
        /// otherwise completely ignored. In other terms, the id is completely arbitrary and exists
        /// only to make it easier for clients to see which of their Asks a particular AskedNote is
        /// responding to.
        ask_id: usize,
    },
}

/// The bytes of a [`Diff`](serde_dif::Diff) describing changes to your hackstead.
/// Embedding these in other structs to serialize them is unfortunately impossible due to the
/// way serde_diff is currently designed, so we miss out on compile time checks that these
/// Vec<u8>s or Strings are, indeed, real [`Diff`s](serde_dif::Diff).
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum EditNote {
    Bincode(Vec<u8>),
    Json(String),
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum ItemAsk {
    /// This Ask should only be performed by privileged users.
    Spawn {
        item_conf: item::Conf,
        amount: usize,
    },
    Throw {
        receiver_id: SteaderId,
        item_ids: Vec<ItemId>,
    },
    Hatch {
        hatchable_item_id: ItemId,
    },
    GotchiNickname {
        item_id: ItemId,
        new_name: String,
    },
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum PlantAsk {
    Summon {
        tile_id: TileId,
        seed_item_id: ItemId,
    },
    Slaughter {
        tile_id: TileId,
    },
    Craft {
        tile_id: TileId,
        recipe_index: usize,
    },
    Rub {
        tile_id: TileId,
        rub_item_id: ItemId,
    },
    Nickname {
        tile_id: TileId,
        new_name: String,
    },
    SkillUnlock {
        tile_id: TileId,
        source_skill_conf: plant::skill::Conf,
        unlock_index: usize,
    },
    KnowledgeSnort {
        tile_id: TileId,
        xp: usize,
    },
}

/// Something the client wants the server to do
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum Ask {
    /// This Ask should only be performed by privileged users.
    KnowledgeSnort {
        xp: usize,
    },
    Plant(PlantAsk),
    Item(ItemAsk),
    TileSummon {
        tile_redeemable_item_id: ItemId,
    },
}

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct AskMessage {
    pub ask: Ask,
    /// The id is only stored on the server to be sent back with the AskedNote, and is
    /// otherwise completely ignored. In other terms, the id is completely arbitrary and exists
    /// only to make it easier for clients to see which of their Asks a particular AskedNote is
    /// responding to.
    pub ask_id: usize,
}

/// Ask the server to do something in an ill-advised way, particularly where the user you are
/// acting on the behalf of cannot be inferred.
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct Beg {
    /// Requires a [`SteaderId`](hcor::id::SteaderId) instead of a [`UserId`](hcor::UserId) to make
    /// this marginally easier to implement server-side; why go out of my way to make doing the
    /// wrong thing easier?
    pub steader_id: SteaderId,
    /// The request to perform.
    pub ask: Ask,
}
