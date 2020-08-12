use crate::{plant, Item, ItemId, Plant, SteaderId, Tile, TileId};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{
    ask, connect, disconnect, register_note_handler, try_note, until, until_map, until_greedy, until_ask_id, until_ask_id_map,
    until_ask_id_map_greedy, WormholeError, WormholeResult,
};

/// How often heartbeat pings are sent
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
/// How long before lack of client response causes a timeout
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long before lack of server response causes a timeout
pub const SERVER_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Format for requesting that a wormhole connection be established
pub struct EstablishWormholeRequest {
    /// The uuid of the user to be associated with this wormhole connection;
    /// only events relevant to this user will be transferred through.
    pub user_id: crate::UserId,
}

type StrResult<T> = Result<T, String>;

#[derive(Serialize, Deserialize, Clone, Debug)]
/// AskedNotes are immediate responses to things explicitly requested by the client using an
/// AskMessage.
pub enum AskedNote {
    KnowledgeSnortResult(StrResult<usize>),
    PlantSummonResult(StrResult<Plant>),
    PlantSlaughterResult(StrResult<Plant>),
    /// Expect a CraftFinish later.
    PlantCraftStartResult(StrResult<plant::Craft>),
    /// Expect a RubFinish later, if the item you applied can wear off.
    PlantRubStartResult(StrResult<Vec<plant::Effect>>),
    TileSummonResult(StrResult<Tile>),
    ItemSpawnResult(StrResult<Vec<Item>>),
    ItemThrowResult(StrResult<Vec<Item>>),
    ItemHatchResult(StrResult<Vec<Item>>),
}

impl AskedNote {
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
            TileSummonResult(Err(e)) => Some(e),
            TileSummonResult(Ok(_)) => None,
            ItemSpawnResult(Err(e)) => Some(e),
            ItemSpawnResult(Ok(_)) => None,
            ItemThrowResult(Err(e)) => Some(e),
            ItemThrowResult(Ok(_)) => None,
            ItemHatchResult(Err(e)) => Some(e),
            ItemHatchResult(Ok(_)) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// Rude Notes aren't responses to any particular Ask from the client,
/// they sort of barge in unnanounced.
pub enum RudeNote {
    ItemThrowReceipt {
        from: SteaderId,
        items: Vec<Item>,
    },
    YieldFinish {
        items: Vec<Item>,
        xp: i32,
        tile_id: TileId,
    },
    CraftFinish {
        items: Vec<Item>,
        xp: i32,
        tile_id: TileId,
    },
    RubEffectFinish {
        effect: plant::Effect,
        tile_id: TileId,
    },
}

/// Like a notification, but cuter; a tidbit of information that the server
/// thinks you might have a special interest in.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Note {
    Rude(RudeNote),
    /// The bytes of a serde_diff::Diff describing changes to your hackstead.
    /// Embedding these in other structs to serialize them is unfortunately impossible due to the
    /// way serde_diff is currently designed, so we miss out on compile time checks that this
    /// Vec<u8> is, indeed, a real Diff.
    Edit(Vec<u8>),
    Asked {
        note: AskedNote,
        ask_id: usize,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ItemAsk {
    Spawn {
        item_archetype_handle: usize,
        amount: usize,
    },
    Throw {
        receiver_id: SteaderId,
        item_ids: Vec<ItemId>,
    },
    Hatch {
        hatchable_item_id: ItemId,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
}

/// Something the client wants the server to do
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Ask {
    KnowledgeSnort { xp: usize },
    Plant(PlantAsk),
    Item(ItemAsk),
    TileSummon {
        tile_redeemable_item_id: ItemId,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AskMessage {
    pub ask: Ask,
    pub ask_id: usize,
}
