use super::RawRecipe;
use super::Recipe;
use crate::{config, item, Hackstead, TileId};
use serde::{Deserialize, Serialize};

#[cfg(feature = "config_verify")]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum RawBuff {
    Neighbor(Box<RawBuff>),
    ExtraTimeTicks(usize),
    ExtraTimeTicksMultiplier(f32),
    Xp(f32),
    YieldSpeedMultiplier(f32),
    YieldSizeMultiplier(f32),
    /// This RawEvalput needs to be verified into an Evalput<item::Conf>
    Yield(config::RawEvalput),
    /// This RawRecipe needs to be verified into a Recipe
    Craft(Vec<RawRecipe>),
    CraftSpeedMultiplier(f32),
    CraftInputReturnChance(f32),
    CraftOutputDoubleChance(f32),
    Art {
        file: String,
        precedence: usize,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum Buff {
    Neighbor(Box<Buff>),
    Art {
        file: String,
        precedence: usize,
    },
    /// Stores the number of extra cycles to add for the duration of the effect
    ExtraTimeTicks(usize),
    ExtraTimeTicksMultiplier(f32),
    Xp(f32),
    // yield
    YieldSpeedMultiplier(f32),
    YieldSizeMultiplier(f32),
    Yield(config::Evalput<item::Conf>),
    // craft
    Craft(Vec<Recipe>),
    CraftSpeedMultiplier(f32),
    CraftInputReturnChance(f32),
    CraftOutputDoubleChance(f32),
}

#[cfg(feature = "config_verify")]
impl config::Verify for RawBuff {
    type Verified = Buff;
    fn verify_raw(self, raw: &config::RawConfig) -> config::VerifResult<Self::Verified> {
        use Buff as B;
        use RawBuff::*;
        Ok(match self {
            Neighbor(b) => B::Neighbor(Box::new(b.verify(raw)?)),
            ExtraTimeTicks(u) => B::ExtraTimeTicks(u),
            ExtraTimeTicksMultiplier(f) => B::ExtraTimeTicksMultiplier(f),
            Xp(f) => B::Xp(f),
            YieldSpeedMultiplier(f) => B::YieldSpeedMultiplier(f),
            YieldSizeMultiplier(f) => B::YieldSizeMultiplier(f),
            Yield(evalput) => B::Yield(evalput.verify(raw)?),
            Craft(recipes) => B::Craft(recipes.verify(raw)?),
            CraftSpeedMultiplier(f) => B::CraftSpeedMultiplier(f),
            CraftInputReturnChance(f) => B::CraftInputReturnChance(f),
            CraftOutputDoubleChance(f) => B::CraftOutputDoubleChance(f),
            Art { file, precedence } => B::Art { file, precedence },
        })
    }

    fn context(&self) -> Option<String> {
        use RawBuff::*;
        Some(format!(
            "in a{} buff",
            match self {
                Neighbor(_) => " neighbor",
                ExtraTimeTicks(_) => "n extra time ticks",
                ExtraTimeTicksMultiplier(_) => " time ticks multiplier",
                Xp(_) => " xp",
                YieldSpeedMultiplier(_) => " yield speed multiplier",
                YieldSizeMultiplier(_) => " yield size multipler",
                Yield(_) => " yield",
                Craft(_) => " craft",
                CraftSpeedMultiplier(_) => " craft speed multiplier",
                CraftInputReturnChance(_) => " craft input return chance",
                CraftOutputDoubleChance(_) => " craft otuput double chance",
                Art { .. } => "n art",
            }
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuffSum {
    pub art: String,
    // time acceleration
    pub total_extra_time_ticks: usize,
    // xp
    pub xp_per_tick: f32,
    // yield
    pub yield_speed_multiplier: f32,
    pub yield_size_multiplier: f32,
    pub yields: config::Evalput<item::Conf>,
    // craft
    pub recipes: Vec<Recipe>,
    pub craft_speed_multiplier: f32,
    pub craft_output_double_chance: f32,
    pub craft_input_return_chance: f32,
}

impl Default for BuffSum {
    fn default() -> Self {
        BuffSum {
            art: Default::default(),
            total_extra_time_ticks: 0,
            xp_per_tick: 0.0,
            yield_speed_multiplier: 1.0,
            yield_size_multiplier: 1.0,
            yields: config::Evalput::Nothing,
            recipes: vec![],
            craft_speed_multiplier: 1.0,
            craft_output_double_chance: 0.0,
            craft_input_return_chance: 0.0,
        }
    }
}

impl BuffSum {
    fn new<'a>(buffs: impl Iterator<Item = &'a Buff>) -> BuffSum {
        struct Art {
            file: Option<String>,
            precedence: usize,
        }

        let mut sum = BuffSum::default();

        let mut art = Art {
            file: None,
            precedence: 0,
        };
        let mut extra_time_ticks = 1.0;
        let mut extra_time_ticks_multiplier = 1.0;
        let mut yields = vec![];

        use Buff::*;
        for buff in buffs {
            match buff {
                Neighbor(_) => {}
                &ExtraTimeTicks(tt) => extra_time_ticks += tt as f32,
                &ExtraTimeTicksMultiplier(m) => extra_time_ticks_multiplier *= m,
                &Xp(xp) => sum.xp_per_tick += xp,
                // yield
                &YieldSpeedMultiplier(speed) => sum.yield_speed_multiplier *= speed,
                &YieldSizeMultiplier(size) => sum.yield_size_multiplier *= size,
                Yield(y) => yields.push(y.clone()),
                // craft
                Craft(recipes) => sum.recipes.append(&mut recipes.clone()),
                &CraftSpeedMultiplier(m) => sum.craft_speed_multiplier *= m,
                &CraftInputReturnChance(ret) => sum.craft_input_return_chance *= ret,
                &CraftOutputDoubleChance(dub) => sum.craft_output_double_chance *= dub,
                &Buff::Art {
                    ref file,
                    precedence,
                } if precedence >= art.precedence => {
                    art = Art {
                        file: Some(file.clone()),
                        precedence,
                    };
                }
                Buff::Art { file, precedence } => log::trace!(
                    "ignoring art because of low precedence: file: {}, precedence: {}",
                    file,
                    precedence
                ),
            }
        }

        sum.art = art.file.expect("no art supplied in plant buffs");
        sum.yields = config::Evalput::All(yields);
        sum.total_extra_time_ticks =
            (extra_time_ticks * extra_time_ticks_multiplier).round() as usize;

        sum
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Source {
    Neighbor(Box<Source>),
    PassiveItemEffect(item::Conf),
    RubbedItemEffect(item::Conf),
    SkillUnlock(super::skill::Conf),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PlantPage {
    buffs: Vec<(Buff, Source)>,
    tile: TileId,
}

#[derive(Debug, Clone, PartialEq)]
struct TrackedBuff {
    tile: TileId,
    buff: Buff,
    source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuffBook {
    plants: Vec<PlantPage>,
    pub sums: std::collections::HashMap<TileId, BuffSum>,
    /// This is the tool we use to spread out our neighbor buffs, or more specifically,
    /// where we store them while we're in the process of spreading them out.
    #[serde(skip)]
    neighbor_knife: Vec<(usize, TrackedBuff)>,
}

impl BuffBook {
    pub fn new(hs: &Hackstead) -> Self {
        let mut s = Self {
            plants: hs
                .plants()
                .into_iter()
                .map(|p| PlantPage {
                    buffs: p.buffs(hs),
                    tile: p.tile_id,
                })
                .collect(),
            neighbor_knife: vec![],
            sums: Default::default(),
        };

        s.spread_neighbors();

        s.sums = s
            .plants
            .iter()
            .map(|p| (p.tile, BuffSum::new(p.buffs.iter().map(|(buff, _)| buff))))
            .collect();

        s
    }

    fn spread_neighbors(&mut self) {
        for (i, (tile, buff, source)) in Self::indexed_buffs(&self.plants) {
            match &buff {
                Buff::Neighbor(b) => self.neighbor_knife.push((
                    i,
                    TrackedBuff {
                        tile,
                        buff: *b.clone(),
                        source: source.clone(),
                    },
                )),
                _ => {}
            }
        }

        if self.neighbor_knife.len() > 0 {
            for (i, neighbor) in self.neighbor_knife.drain(..) {
                for page in self.plants.iter_mut() {
                    if page.tile == neighbor.tile {
                        page.buffs.swap_remove(i);
                    } else {
                        page.buffs.push((
                            neighbor.buff.clone(),
                            Source::Neighbor(Box::new(neighbor.source.clone())),
                        ));
                    }
                }
            }

            self.spread_neighbors()
        }
    }

    fn indexed_buffs(
        plants: &[PlantPage],
    ) -> impl Iterator<Item = (usize, (TileId, &Buff, &Source))> {
        plants.iter().flat_map(|pp| {
            pp.buffs
                .iter()
                .map(move |(buff, source)| (pp.tile, buff, source))
                .enumerate()
        })
    }
}
