use super::{EffectId, TileId};
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub enum TimerKind {
    Yield,
    Craft { recipe_index: usize },
    Rub { effect_id: EffectId },
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub enum Lifecycle {
    // when this timer finishes, it restarts again.
    Perennial { duration: f32 },
    // this timer runs once, then, kaputt.
    Annual,
}

impl Lifecycle {
    pub fn is_perennial(&self) -> bool {
        matches!(self, Lifecycle::Perennial { .. } )
    }

    pub fn is_annual(&self) -> bool {
        matches!(self, Lifecycle::Annual)
    }
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct Timer {
    pub until_finish: f32,
    pub lifecycle: Lifecycle,
    pub tile_id: TileId,
    pub kind: TimerKind,
}
