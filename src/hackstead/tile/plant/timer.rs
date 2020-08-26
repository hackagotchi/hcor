use super::{RubEffectId, TileId};
use crate::IdentifiesTile;
use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;
use std::fmt;

#[derive(SerdeDiff, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
#[serde_diff(opaque)]
pub struct TimerId(pub uuid::Uuid);

impl fmt::Display for TimerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub enum TimerKind {
    Yield,
    Craft { recipe_index: usize },
    Rub { effect_id: RubEffectId },
    Xp
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
pub struct ServerTimer {
    pub kind: TimerKind,
    pub tile_id: TileId,
    pub timer_id: TimerId,
    pub value: f32,
    pub predicted_next: f32,
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct ClientTimer {
    pub timer_id: TimerId,
    pub value: f32,
    pub rate: f32,
}

#[derive(Debug, Clone, Copy, SerdeDiff, Serialize, Deserialize, PartialEq)]
pub struct SharedTimer {
    pub timer_id: TimerId,
    pub duration: f32,
    pub lifecycle: Lifecycle,
    pub tile_id: TileId,
    pub kind: TimerKind,
}

impl SharedTimer {
    pub fn new(lifecycle: Lifecycle, kind: TimerKind, t: impl IdentifiesTile, duration: f32) -> Self {
        Self {
            timer_id: TimerId(uuid::Uuid::new_v4()),
            duration,
            lifecycle,
            tile_id: t.tile_id(),
            kind,
        }
    }

    pub fn for_skills(tile_id: TileId, conf: super::Conf, skills: &[super::skill::Conf]) -> Vec<SharedTimer> {
        use super::Buff;
        
        let mut timers = vec![];
        let buffs: Vec<_> = skills.iter().flat_map(|s| s.effects.iter()).filter_map(|e| e.kind.buff()).collect();

        if buffs.iter().any(|b| matches!(b, Buff::Xp(_))) {
            timers.push(SharedTimer::new(
                Lifecycle::Perennial {
                    duration: 1000.0,
                },
                TimerKind::Xp,
                tile_id,
                1000.0,
            ))
        }

        if buffs.iter().any(|b| matches!(b, Buff::Yield(_))) {
            if let Some(duration) = conf.base_yield_duration {
                timers.push(SharedTimer::new(
                    Lifecycle::Perennial { duration },
                    TimerKind::Yield,
                    tile_id,
                    duration,
                ))
            }
        }

        timers
    }
}
