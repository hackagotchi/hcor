use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

#[cfg(test)]
mod test;

pub type ConfigResult<T> = Result<T, ConfigError>;
#[derive(Debug, Clone)]
pub enum ConfigError {
    UnknownPossessionArchetypeName(String),
    UnknownPossessionArchetype(Archetype),
    UnknownPossessionArchetypeHandle(ArchetypeHandle),
    UnknownEffectArchetypeHandle {
        item: ArchetypeHandle,
        effect: ArchetypeHandle,
    },
}
impl std::error::Error for ConfigError {}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ConfigError::*;
        match self {
            UnknownPossessionArchetypeName(name) => {
                write!(f, "no archetype in config by the name of {:?}", name)
            }
            UnknownPossessionArchetype(a) => {
                write!(f, "no possession archetype in config {:#?}", a)
            }
            UnknownPossessionArchetypeHandle(a) => {
                write!(f, "no possession archetype in config with handle {}", a)
            }
            UnknownEffectArchetypeHandle { item, effect } => write!(
                f,
                "no plant rub effect archetype in config with handle {} on item {}",
                effect, item
            ),
        }
    }
}

pub type LootTableHandle = usize;
pub type LootTable = Vec<(CountProbability, String)>;

pub trait SpawnTableRow {
    type Output;

    /// How likely is this row to occur?
    /// NOTE: since this is currently only used for calculating
    /// percentiles for eggs, perhaps we want to replace this with
    /// some configurably-defined "desireability" factor?
    /// Rarity does not strictly correlate with the value of something.
    fn rarity(&self) -> f32;

    /// How many of this item should be outputted?
    fn count(&self, rng: &mut impl rand::RngCore) -> usize;

    /// What does this row actually produce?
    fn output(&self, index: usize) -> Self::Output;
}

impl<Handle: Clone> SpawnTableRow for (CountProbability, Handle) {
    type Output = Handle;

    fn rarity(&self) -> f32 {
        (self.0).0
    }
    fn count(&self, rng: &mut impl rand::RngCore) -> usize {
        self.0.gen_count(rng)
    }
    fn output(&self, _index: usize) -> Self::Output {
        self.1.clone()
    }
}

/// Spawns an iterator of output dictated by various rows in a table of things to spawn.
pub fn spawn<'rng, Row: SpawnTableRow>(
    table: &'rng [Row],
    rng: &'rng mut impl rand::RngCore,
) -> impl Iterator<Item = Row::Output> + 'rng {
    table
        .iter()
        .flat_map(move |row| (0..row.count(rng)).map(move |i| row.output(i)))
}

/// Quite similar to spawn(), but it also returns a list of the
/// leading spawn rates for the rows in the table the spawn qualified for.
pub fn spawn_with_percentile<Row: SpawnTableRow>(
    table: &[Row],
    rng: &mut impl rand::RngCore,
) -> (Vec<Row::Output>, f32) {
    let (handles, rows): (Vec<Vec<Row::Output>>, Vec<f32>) = table
        .iter()
        .filter_map(|row| {
            let count = row.count(rng);

            if count == 0 {
                None
            } else {
                Some((
                    (0..count).map(move |i| row.output(i)).collect::<Vec<_>>(),
                    row.rarity(),
                ))
            }
        })
        .unzip();

    (handles.into_iter().flat_map(|h| h.into_iter()).collect(), {
        let best_roll: f32 = table.iter().map(|row| row.rarity()).product();
        1.0 - ((rows.into_iter().product::<f32>() - best_roll) / (1.0 - best_roll))
    })
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub special_users: Vec<String>,
    pub profile_archetype: ProfileArchetype,
    pub plant_archetypes: Vec<PlantArchetype>,
    pub possession_archetypes: Vec<Archetype>,
}

impl Config {
    pub fn welcome_gifts(&self) -> impl Iterator<Item = &Archetype> {
        self.possession_archetypes.iter().filter(|a| a.welcome_gift)
    }

    pub fn seeds(&self) -> impl Iterator<Item = (&SeedArchetype, &Archetype)> {
        self.possession_archetypes
            .iter()
            .filter_map(|a| Some((a.seed.as_ref()?, a)))
    }

    pub fn gotchi(&self) -> impl Iterator<Item = (&GotchiArchetype, &Archetype)> {
        self.possession_archetypes
            .iter()
            .filter_map(|a| Some((a.gotchi.as_ref()?, a)))
    }

    pub fn land_unlockers(&self) -> impl Iterator<Item = (&LandUnlock, &Archetype)> {
        self.possession_archetypes
            .iter()
            .filter_map(|a| Some((a.unlocks_land.as_ref()?, a)))
    }

    pub fn find_plant<S: AsRef<str>>(&self, name: &S) -> Result<&PlantArchetype, ConfigError> {
        self.plant_archetypes
            .iter()
            .find(|x| name.as_ref() == x.name)
            .ok_or_else(|| ConfigError::UnknownPossessionArchetypeName(name.as_ref().to_string()))
    }

    pub fn find_plant_handle<S: AsRef<str>>(
        &self,
        name: &S,
    ) -> Result<ArchetypeHandle, ConfigError> {
        self.plant_archetypes
            .iter()
            .position(|x| name.as_ref() == x.name)
            .ok_or_else(|| ConfigError::UnknownPossessionArchetypeName(name.as_ref().to_string()))
            .map(|x| x as ArchetypeHandle)
    }

    pub fn find_possession<S: AsRef<str>>(&self, name: &S) -> Result<&Archetype, ConfigError> {
        self.possession_archetypes
            .iter()
            .find(|x| name.as_ref() == x.name)
            .ok_or_else(|| ConfigError::UnknownPossessionArchetypeName(name.as_ref().to_string()))
    }

    pub fn item(&self, item_arch: ArchetypeHandle) -> ConfigResult<&Archetype> {
        self.possession_archetypes
            .get(item_arch as usize)
            .ok_or(ConfigError::UnknownPossessionArchetypeHandle(item_arch))
    }

    pub fn plant_rub_effect(
        &self,
        item_arch: ArchetypeHandle,
        effect_arch: ArchetypeHandle,
    ) -> ConfigResult<&PlantRubEffect> {
        self.item(item_arch)?
            .plant_rub_effects
            .get(effect_arch as usize)
            .ok_or(ConfigError::UnknownEffectArchetypeHandle {
                item: item_arch,
                effect: effect_arch,
            })
    }

    pub fn possession_name_to_handle<S: AsRef<str>>(
        &self,
        name: &S,
    ) -> Result<ArchetypeHandle, ConfigError> {
        self.possession_archetypes
            .iter()
            .position(|x| name.as_ref() == x.name)
            .ok_or_else(|| ConfigError::UnknownPossessionArchetypeName(name.as_ref().to_string()))
            .map(|x| x as ArchetypeHandle)
    }

    pub fn possession_archetype_to_handle(&self, a: &Archetype) -> ConfigResult<ArchetypeHandle> {
        self.possession_archetypes
            .iter()
            .position(|x| x == a)
            .map(|x| x as ArchetypeHandle)
            .ok_or_else(|| ConfigError::UnknownPossessionArchetype(a.clone()))
    }
}

// I should _really_ use a different version of this for PlantArchetypes and PossessionArchetypes ...
pub type ArchetypeHandle = usize;

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = {
        pub fn f<T: DeserializeOwned>(p: &'static str) -> T {
            serde_json::from_str(
                &std::fs::read_to_string(format!(
                    concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/config/{}.json",
                    ),
                    p
                ))
                .unwrap_or_else(|e| panic!("opening {}: {}", p, e))
            )
            .unwrap_or_else(|e| panic!("parsing {}: {}", p, e))
        }

        Config {
            special_users: f("special_users"),
            ..f("content")
        }
    };
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProfileArchetype {
    pub advancements: AdvancementSet<HacksteadAdvancementSum>,
}

pub type HacksteadAdvancement = Advancement<HacksteadAdvancementSum>;
pub type HacksteadAdvancementSet = AdvancementSet<HacksteadAdvancementSum>;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HacksteadAdvancementKind {
    Land { pieces: usize },
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HacksteadAdvancementSum {
    pub land: usize,
    pub xp: usize,
}
impl AdvancementSum for HacksteadAdvancementSum {
    type Kind = HacksteadAdvancementKind;

    fn new(unlocked: &[&Advancement<Self>]) -> Self {
        Self {
            xp: unlocked.iter().fold(0, |a, c| a + c.xp),
            land: unlocked
                .iter()
                .map(|k| match k.kind {
                    HacksteadAdvancementKind::Land { pieces } => pieces,
                })
                .sum(),
        }
    }

    fn filter_base(_a: &Advancement<Self>) -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlantFilter<Handle> {
    Only(Vec<Handle>),
    Not(Vec<Handle>),
    All,
}
impl PlantFilter<String> {
    pub fn lookup_handles(&self) -> Result<PlantFilter<ArchetypeHandle>, ConfigError> {
        use PlantFilter::*;

        Ok(match self {
            Only(these) => Only(
                these
                    .iter()
                    .map(|h| CONFIG.find_plant_handle(h))
                    .collect::<Result<_, _>>()?,
            ),
            Not(these) => Not(these
                .iter()
                .map(|h| CONFIG.find_plant_handle(h))
                .collect::<Result<_, _>>()?),
            All => All,
        })
    }
}
impl<Handle> PlantFilter<Handle> {
    pub fn allows<T>(&self, t: &T) -> bool
    where
        Handle: std::borrow::Borrow<T>,
        T: PartialEq + ?Sized,
    {
        use PlantFilter::*;

        match self {
            Only(these) => these.iter().any(|h| h.borrow() == t),
            Not(these) => !these.iter().any(|h| h.borrow() == t),
            All => true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SelectivePlantAdvancement {
    pub for_plants: PlantFilter<String>,
    pub advancement: PlantAdvancement,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GotchiArchetype {
    pub base_happiness: usize,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SeedArchetype {
    pub grows_into: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LandUnlock {
    pub requires_xp: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlantRubEffectKind {
    PlantAdvancement(PlantAdvancement),
    TurnsPlantInto(String),
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlantRubEffect {
    pub short_description: String,
    pub duration: Option<f32>,
    pub plant_filter: PlantFilter<String>,
    pub kind: PlantRubEffectKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Archetype {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub gotchi: Option<GotchiArchetype>,
    #[serde(default)]
    pub seed: Option<SeedArchetype>,
    #[serde(default)]
    pub unlocks_land: Option<LandUnlock>,
    #[serde(default)]
    pub welcome_gift: bool,
    #[serde(default)]
    pub passive_plant_effects: Vec<SelectivePlantAdvancement>,
    #[serde(default)]
    pub plant_rub_effects: Vec<PlantRubEffect>,
    #[serde(default)]
    pub hatch_table: Option<LootTable>,
}
impl Archetype {
    pub fn rub_effects_for_plant<'a>(
        &'a self,
        p: &'a str,
    ) -> impl Iterator<Item = &'a PlantRubEffect> {
        self.plant_rub_effects
            .iter()
            .filter(move |e| e.plant_filter.allows(p))
    }

    pub fn rub_effects_for_plant_indexed<'a>(
        &'a self,
        p: &'a str,
    ) -> impl Iterator<Item = (usize, &'a PlantRubEffect)> {
        self.plant_rub_effects
            .iter()
            .enumerate()
            .filter(move |(_, e)| e.plant_filter.allows(p))
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use crate::{
        client::{ClientError, ClientResult},
        wormhole::{ask, until_ask_id_map, AskedNote, ItemAsk},
        Ask, Item,
    };

    impl Archetype {
        pub async fn spawn_some(&self, amount: usize) -> ClientResult<Vec<Item>> {
            let a = Ask::Item(ItemAsk::Spawn {
                item_archetype_handle: CONFIG
                    .possession_archetype_to_handle(self)
                    .expect("archetype not found in config"),
                amount,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::ItemSpawnResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a, "ItemSpawn", e))
        }

        pub async fn spawn(&self) -> ClientResult<Item> {
            let a = Ask::Item(ItemAsk::Spawn {
                item_archetype_handle: CONFIG
                    .possession_archetype_to_handle(self)
                    .expect("archetype not found in config"),
                amount: 1,
            });

            let ask_id = ask(a.clone()).await?;

            until_ask_id_map(ask_id, |n| match n {
                AskedNote::ItemSpawnResult(r) => Some(r),
                _ => None,
            })
            .await?
            .map_err(|e| ClientError::bad_ask(a.clone(), "ItemSpawn", e))?
            .pop()
            .ok_or_else(|| {
                ClientError::bad_ask(a, "ItemSpawn", "Asked for one item, got none".to_string())
            })
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlantArchetype {
    pub name: String,
    pub base_yield_duration: Option<f32>,
    pub advancements: AdvancementSet<PlantAdvancementSum>,
}
impl Eq for PlantArchetype {}
impl PartialEq for PlantArchetype {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Hash for PlantArchetype {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
pub type PlantAdvancement = Advancement<PlantAdvancementSum>;
pub type PlantAdvancementSet = AdvancementSet<PlantAdvancementSum>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RecipeMakes<Handle: Clone> {
    Just(usize, Handle),
    Nothing,
    OneOf(Vec<(f32, RecipeMakes<Handle>)>),
    AllOf(Vec<(usize, Handle)>),
}
impl<Handle: Clone> RecipeMakes<Handle> {
    fn pick_one_weighted_of<T: Clone>(from: &[(f32, T)]) -> T {
        use rand::Rng;
        let mut x: f32 = rand::thread_rng().gen_range(0.0, 1.0);
        from.iter()
            .find_map(|(chance, h)| {
                x -= chance;
                if x < 0.0 {
                    Some(h)
                } else {
                    None
                }
            })
            .unwrap()
            .clone()
    }

    /// Returns one possible output, randomly (but properly weighted)
    /// if more than one is possible.
    pub fn any(&self) -> Option<Handle> {
        use RecipeMakes::*;

        match self {
            OneOf(these) => Self::pick_one_weighted_of(these).any(),
            Just(_, h) => Some(h.clone()),
            AllOf(these) => {
                let total = these.iter().map(|(count, _)| *count).sum::<usize>() as f32;
                Some(Self::pick_one_weighted_of(
                    &these
                        .iter()
                        .map(|(count, h)| (*count as f32 / total, h.clone()))
                        .collect::<Vec<_>>(),
                ))
            }
            Nothing => None,
        }
    }

    /// A list of everything that could possibly come from this recipe
    pub fn all(&self) -> Vec<(Handle, usize)> {
        use RecipeMakes::*;

        match self {
            OneOf(these) => Self::pick_one_weighted_of(these).all(),
            Just(count, h) => [(h.clone(), *count)].to_vec(),
            AllOf(these) => these.iter().map(|(count, h)| (h.clone(), *count)).collect(),
            Nothing => vec![],
        }
    }

    /// A proper output given the constraints of this recipe;
    /// for example, OneOfWeighted will always return one of the possibilities,
    /// properly weighted.
    pub fn output(self) -> Vec<Handle> {
        use RecipeMakes::*;

        match &self {
            OneOf(these) => Self::pick_one_weighted_of(these).output(),
            Just(count, _) => (0..*count)
                .map(|_| {
                    self.any()
                        .expect("RecipeMakes::Just.any() can't return None")
                })
                .collect(),
            AllOf(_) => self
                .all()
                .into_iter()
                .flat_map(|(what, count)| (0..count).map(move |_| what.clone()))
                .collect(),
            Nothing => vec![],
        }
    }
}
impl fmt::Display for RecipeMakes<&'static Archetype> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::frontend::emojify;
        use RecipeMakes::*;

        match self {
            Just(1, x) => write!(f, "a {} _{}_", emojify(&x.name), x.name),
            Just(n, x) => write!(f, "*{}* {} _{}_", n, emojify(&x.name), x.name),
            OneOf(these) => write!(
                f,
                "one of these:\n{}",
                these
                    .iter()
                    .map(|(chance, what)| { format!("{} (*{:.2}%* chance)", what, chance * 100.0) })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            AllOf(these) => write!(
                f,
                "all of the following:\n{}",
                these
                    .iter()
                    .map(|(count, what)| {
                        format!("*{}* {} _{}_", emojify(&what.name), what.name, count)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Nothing => Ok(()),
        }
    }
}

impl RecipeMakes<ArchetypeHandle> {
    pub fn lookup_handles(self) -> Option<RecipeMakes<&'static Archetype>> {
        use RecipeMakes::*;

        fn lookup(ah: ArchetypeHandle) -> Option<&'static Archetype> {
            CONFIG.possession_archetypes.get(ah as usize)
        }

        Some(match self {
            Just(n, ah) => Just(n, lookup(ah)?),
            OneOf(l) => OneOf(
                l.into_iter()
                    .map(|(c, recipe)| Some((c, recipe.lookup_handles()?)))
                    .collect::<Option<_>>()?,
            ),
            AllOf(l) => AllOf(
                l.into_iter()
                    .map(|(c, ah)| Some((c, lookup(ah)?)))
                    .collect::<Option<_>>()?,
            ),
            Nothing => Nothing,
        })
    }
}

impl RecipeMakes<String> {
    pub fn find_handles(self) -> Result<RecipeMakes<ArchetypeHandle>, ConfigError> {
        use RecipeMakes::*;

        fn find(name: String) -> Result<ArchetypeHandle, ConfigError> {
            CONFIG.possession_name_to_handle(&name)
        }

        Ok(match self {
            Just(n, ah) => Just(n, find(ah)?),
            OneOf(l) => OneOf(
                l.into_iter()
                    .map(|(c, recipe)| Ok((c, recipe.find_handles()?)))
                    .collect::<Result<_, _>>()?,
            ),
            AllOf(l) => AllOf(
                l.into_iter()
                    .map(|(c, ah)| Ok((c, find(ah)?)))
                    .collect::<Result<_, _>>()?,
            ),
            Nothing => Nothing,
        })
    }
}

/// Recipe is generic over the way Archetypes are referred to
/// to make it easy to use Strings in the configs and ArchetypeHandles
/// at runtime
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Recipe<Handle: Clone> {
    pub title: Option<String>,
    pub explanation: Option<String>,
    pub needs: Vec<(usize, Handle)>,
    pub makes: RecipeMakes<Handle>,
    #[serde(default)]
    pub destroys_plant: bool,
    pub time: f32,
    pub xp: (usize, usize),
}
impl Recipe<ArchetypeHandle> {
    pub fn satisfies(&self, inv: &[crate::Item]) -> bool {
        self.needs.iter().copied().all(|(count, ah)| {
            let has = inv.iter().filter(|x| x.archetype_handle == ah).count();
            count <= has
        })
    }
    pub fn lookup_handles(self) -> Option<Recipe<&'static Archetype>> {
        let Recipe {
            time,
            destroys_plant,
            title,
            explanation,
            makes,
            needs,
            xp,
        } = self;
        Some(Recipe {
            makes: makes.lookup_handles()?,
            needs: needs
                .into_iter()
                .map(|(n, x)| Some((n, CONFIG.possession_archetypes.get(x as usize)?)))
                .collect::<Option<Vec<(_, &Archetype)>>>()?,
            time,
            destroys_plant,
            title,
            explanation,
            xp,
        })
    }
}
impl Recipe<&Archetype> {
    pub fn title(&self) -> String {
        self.title.clone().unwrap_or_else(|| {
            self.makes
                .any()
                .map(|e| format!("{} {}", crate::frontend::emojify(&e.name), e.name))
                .unwrap_or_else(|| "Nothing".to_string())
        })
    }
    pub fn explanation(&self) -> String {
        self.explanation.clone().unwrap_or_else(|| {
            self.makes
                .any()
                .map(|e| e.description.clone())
                .unwrap_or_else(|| "Nothing".to_string())
        })
    }
}

/// a number between these two bounds is chosen (the first is the lower bound, the second is
/// the higher bound). The floating point number is then split into its fractional and integral
/// counterparts. The integral counterpart is the base number of items to award, and the
/// fractional counterpart becomes a probability that an extra item is awarded. For example,
/// 1.99 is one item guaranteed, with a 99% chance of a second item being awarded.
pub type AmountBounds = (f32, f32);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CountProbability(pub f32, pub AmountBounds);
impl CountProbability {
    pub fn gen_count<R: rand::Rng>(self, rng: &mut R) -> usize {
        let Self(guard, (lo, hi)) = self;
        if rng.gen_range(0.0, 1.0) < guard {
            let chance = rng.gen_range(lo, hi);
            let base = chance.floor();
            let extra = if rng.gen_range(0.0, 1.0) < chance - base {
                1
            } else {
                0
            };
            base as usize + extra
        } else {
            0
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Describes the output of a plant as may occur at set intervals.
/// Yields occur conditionally, and produce a randomly chosen (within specific bounds) amount of a
/// specified item, and some number of experience points, also randomly chosen from within
/// arbitrary bounds.
pub struct Yield<Handle> {
    /// The chance this yield has of even occuring, in the domain [0.0, 1.0].
    /// Note that yields which do not occur yield neither xp nor items.
    pub chance: f32,
    /// The number of items to produce, see the documentation on AmountBounds for more information.
    pub amount: AmountBounds,
    /// This field dictates how much less xp is earned per item in a yield.
    /// The formula used is xp / (i * dropoff), where xp is an amount in the bounds below,
    /// and i is the index of the item.
    pub dropoff: f32,
    /// An upper and lower bound for a random amount of xp to be awarded should this yield occur
    /// (as determined by the chance field).
    /// The xp is awarded on a per item basis, although dropoff can be used to ensure less and
    /// lesss xp is awarded per item.
    pub xp: (usize, usize),
    /// What item this yield outputs, should it occur as according to the chance field on this
    /// struct. Note that the amount of this item to be output is determined by the amount field.
    pub yields: Handle,
}
impl Yield<String> {
    /// Takes ownership of an existing yield, producing an identical one which contains
    /// a handle to the type of item the yield may output, which is guaranteed to point
    /// to an Archetype which exists in the configuration, unlike the String which may
    /// be invalid. If the String which specifies the possible output is invalid, an error is
    /// returned.
    fn lookup_handles(self) -> Result<Yield<ArchetypeHandle>, ConfigError> {
        let Self {
            chance,
            amount,
            xp,
            dropoff,
            yields,
        } = self;

        Ok(Yield {
            chance,
            amount,
            xp,
            dropoff,
            yields: CONFIG.possession_name_to_handle(&yields)?,
        })
    }
}

impl<Handle: Clone> SpawnTableRow for Yield<Handle> {
    /// Yields also return a certain amount of xp,
    /// alongside something you can use to spawn an item with.
    type Output = (Handle, usize);

    fn rarity(&self) -> f32 {
        self.chance
    }
    fn count(&self, rng: &mut impl rand::RngCore) -> usize {
        CountProbability(self.chance, self.amount).gen_count(rng)
    }
    fn output(&self, index: usize) -> Self::Output {
        (self.yields.clone(), {
            use rand::Rng;

            let (lo, hi) = self.xp;
            (rand::thread_rng().gen_range(lo, hi) as f32 / ((index + 1) as f32 * self.dropoff))
                .round() as usize
        })
    }
}

/// A leading chance, then a min and max percent of things returned.
/// Should be reminiscent of loot tables.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlantAdvancementKind {
    Neighbor(Box<PlantAdvancementKind>),
    /// Stores the number of extra cycles to add for the duration of the effect
    ExtraTimeTicks(usize),
    TimeTicksMultiplier(f32),
    Xp(f32),
    YieldSpeedMultiplier(f32),
    YieldSizeMultiplier(f32),
    Yield(Vec<Yield<String>>),
    Craft(Vec<Recipe<String>>),
    CraftSpeedMultiplier(f32),
    CraftReturnChance(f32),
    DoubleCraftYield(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(bound(deserialize = ""))]
pub struct PlantAdvancementSum {
    // time acceleration
    pub total_extra_time_ticks: u128,
    // xp
    pub xp: usize,
    pub xp_multiplier: usize,
    // yield
    pub yield_speed_multiplier: f32,
    pub yield_size_multiplier: f32,
    pub yields: Vec<Yield<ArchetypeHandle>>,
    // craft
    pub double_craft_yield_chance: f32,
    pub crafting_speed_multiplier: f32,
    pub craft_return_chance: f32,
    pub recipes: Vec<Recipe<ArchetypeHandle>>,
}
impl AdvancementSum for PlantAdvancementSum {
    type Kind = PlantAdvancementKind;

    fn new(unlocked: &[&Advancement<Self>]) -> Self {
        use PlantAdvancementKind::*;

        // time
        let mut time_ticks_multiplier: f32 = 1.0;
        let mut extra_time_ticks: usize = 0;
        // xp
        let mut xp = 0;
        let mut xp_multiplier = 0;
        // yield
        let mut yield_speed_multiplier = 1.0;
        let mut yield_size_multiplier = 1.0;
        let mut yields = vec![];
        // craft
        let mut double_craft_yield_chance = 0.0;
        let mut crafting_speed_multiplier = 1.0;
        let mut craft_return_chance = 0.0;
        let mut recipes = vec![];

        for k in unlocked.iter() {
            xp += k.xp;

            // apply neighbor upgrades as if they weren't neighbor upgrades :D
            let kind = match &k.kind {
                Neighbor(n) => &**n,
                other => other,
            };

            match kind {
                // misc
                Neighbor(..) => {}
                &TimeTicksMultiplier(multiplier) => time_ticks_multiplier *= multiplier,
                &ExtraTimeTicks(extra) => extra_time_ticks += extra,

                // xp
                &Xp(xp_gain) => xp_multiplier += xp_gain as usize,

                // yield
                &YieldSpeedMultiplier(multiplier) => yield_speed_multiplier *= multiplier,
                &YieldSizeMultiplier(multiplier) => yield_size_multiplier *= multiplier,
                Yield(resources) => yields.append(
                    &mut resources
                        .clone()
                        .into_iter()
                        .map(|y| y.lookup_handles())
                        .collect::<Result<Vec<_>, ConfigError>>()
                        .expect("couldn't find archetype for advancement yield"),
                ),

                // craft
                &CraftReturnChance(chance) => craft_return_chance += chance,
                &CraftSpeedMultiplier(multiplier) => crafting_speed_multiplier *= multiplier,
                &DoubleCraftYield(chance) => double_craft_yield_chance += chance,
                Craft(new_recipes) => recipes.append(
                    &mut new_recipes
                        .clone()
                        .into_iter()
                        .map(|r| {
                            let Recipe {
                                makes,
                                needs,
                                time,
                                destroys_plant,
                                title,
                                explanation,
                                xp,
                            } = r;
                            Ok(Recipe {
                                makes: makes.find_handles()?,
                                needs: needs
                                    .iter()
                                    .map(|(c, s)| Ok((*c, CONFIG.possession_name_to_handle(s)?)))
                                    .collect::<Result<Vec<_>, ConfigError>>()?,
                                time,
                                destroys_plant,
                                title,
                                explanation,
                                xp,
                            })
                        })
                        .collect::<Result<Vec<_>, ConfigError>>()
                        .expect("couldn't find archetype for crafting advancement"),
                ),
            }
        }

        yields = yields
            .into_iter()
            .map(|mut y| {
                y.chance = (y.chance * yield_size_multiplier).min(1.0);

                let (lo, hi) = y.amount;
                y.amount = (lo * yield_size_multiplier, hi * yield_size_multiplier);

                y
            })
            .collect();

        Self {
            total_extra_time_ticks: ((extra_time_ticks as f32) * time_ticks_multiplier).ceil()
                as u128,
            // xp
            xp,
            xp_multiplier,
            // yield
            yield_speed_multiplier,
            yield_size_multiplier,
            yields,
            // craft
            crafting_speed_multiplier,
            double_craft_yield_chance,
            craft_return_chance,
            recipes,
        }
    }

    // ignore your neighbor bonuses you give out
    fn filter_base(a: &Advancement<Self>) -> bool {
        match &a.kind {
            PlantAdvancementKind::Neighbor(..) => false,
            _ => true,
        }
    }
}

pub trait AdvancementSum: DeserializeOwned + Serialize + PartialEq + fmt::Debug {
    type Kind: DeserializeOwned + Serialize + fmt::Debug + Clone + PartialEq;

    fn new(unlocked: &[&Advancement<Self>]) -> Self;
    fn filter_base(a: &Advancement<Self>) -> bool;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(bound(deserialize = ""))]
pub struct Advancement<S: AdvancementSum> {
    pub kind: S::Kind,
    pub xp: usize,
    pub art: String,
    pub title: String,
    pub description: String,
    pub achiever_title: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(bound(deserialize = ""))]
pub struct AdvancementSet<S: AdvancementSum> {
    pub base: Advancement<S>,
    pub rest: Vec<Advancement<S>>,
}
#[allow(dead_code)]
impl<S: AdvancementSum> AdvancementSet<S> {
    pub fn all(&self) -> impl Iterator<Item = &Advancement<S>> {
        std::iter::once(&self.base).chain(self.rest.iter())
    }
    pub fn unlocked(&self, xp: usize) -> impl Iterator<Item = &Advancement<S>> {
        std::iter::once(&self.base).chain(self.rest.iter().take(self.current_position(xp)))
    }

    pub fn get(&self, index: usize) -> Option<&Advancement<S>> {
        if index == 0 {
            Some(&self.base)
        } else {
            self.rest.get(index - 1)
        }
    }

    pub fn increase_xp(&self, xp: &mut usize, amt: usize) -> Option<&Advancement<S>> {
        *xp += amt;
        self.next(xp.checked_sub(1).unwrap_or(0))
            .filter(|&x| self.next(*xp).map(|n| *x != *n).unwrap_or(false))
    }

    /// All currently unlocked advancements, filtered according to the sum implementation,
    /// with extra advancements tacked onto the end, summed into a single struct which
    /// compiles all of the bonuses into one easily accessible place
    pub fn sum<'a>(
        &'a self,
        xp: usize,
        extra_advancements: impl Iterator<Item = &'a Advancement<S>>,
    ) -> S {
        S::new(
            &self
                .unlocked(xp)
                .filter(|&x| S::filter_base(x))
                .chain(extra_advancements)
                .collect::<Vec<_>>(),
        )
    }

    /// Unfiltered advancements
    pub fn raw_sum<'a>(
        &'a self,
        xp: usize,
        extra_advancements: impl Iterator<Item = &'a Advancement<S>>,
    ) -> S {
        S::new(
            &self
                .unlocked(xp)
                .chain(extra_advancements)
                .collect::<Vec<_>>(),
        )
    }

    /// A sum of all possible advancements
    pub fn max<'a>(&'a self, extra_advancements: impl Iterator<Item = &'a Advancement<S>>) -> S {
        S::new(
            &self
                .all()
                .filter(|&x| S::filter_base(x))
                .chain(extra_advancements)
                .collect::<Vec<_>>(),
        )
    }

    pub fn current(&self, xp: usize) -> &Advancement<S> {
        self.get(self.current_position(xp)).unwrap_or(&self.base)
    }

    pub fn next(&self, xp: usize) -> Option<&Advancement<S>> {
        self.get(self.current_position(xp) + 1)
    }

    pub fn current_position(&self, xp: usize) -> usize {
        let mut state = 0;
        self.all()
            .position(|x| {
                state += x.xp;
                state > xp
            })
            .unwrap_or(self.rest.len() + 1)
            .saturating_sub(1)
    }
}

/// In the config, you can specify the names of archetypes.
/// If you're Rishi, you might spell one of those names wrong.
/// This function helps you make sure you didn't do that.
pub fn check_archetype_name_matches(config: &Config) -> Result<(), String> {
    for a in config.possession_archetypes.iter() {
        if let Some(sa) = &a.seed {
            if config.find_plant(&sa.grows_into).is_err() {
                return Err(format!(
                    "seed archetype {:?} claims it grows into unknown plant archetype {:?}",
                    a.name, sa.grows_into,
                ));
            }
        }
        if let Some(table) = &a.hatch_table {
            for (_, spawn) in table.iter() {
                if config.find_possession(spawn).is_err() {
                    return Err(format!(
                        "gotchi archetype {:?} claims it hatches into unknown possession archetype {:?}",
                        a.name,
                        spawn,
                    ));
                }
            }
        }
        for SelectivePlantAdvancement { for_plants, .. } in &a.passive_plant_effects {
            if let Err(e) = for_plants.lookup_handles() {
                return Err(format!(
                    "possession archetype {:?} keep plants error: {}\nkeep plants: {:?}",
                    a.name, e, for_plants,
                ));
            }
        }
        for ef in a.plant_rub_effects.iter() {
            if let Err(e) = ef.plant_filter.lookup_handles() {
                return Err(format!(
                    concat!(
                        "possession archetype {:?} ",
                        "plant rub desc {:?} ",
                        "keep plants error: {}\n",
                        "keep plants: {:?}",
                    ),
                    a.name, ef.short_description, e, ef.plant_filter,
                ));
            }
        }
    }

    for (arch, adv) in config
        .plant_archetypes
        .iter()
        .flat_map(|arch| arch.advancements.all().map(move |adv| (arch.clone(), adv)))
    {
        use PlantAdvancementKind::*;

        match &adv.kind {
            Yield(yields) => {
                for y in yields.iter() {
                    if config.find_possession(&y.yields).is_err() {
                        return Err(format!(
                            "Yield advancement {:?} for plant {:?} includes unknown resource {:?}",
                            adv.title, arch.name, y.yields,
                        ));
                    }
                }
            }
            Craft(recipes) => {
                for Recipe { makes, needs, .. } in recipes.iter() {
                    for (resource, _) in makes.all().iter() {
                        if config.find_possession(resource).is_err() {
                            return Err(format!(
                                "Crafting advancement {:?} for plant {:?} produces unknown resource {:?}",
                                adv.title,
                                arch.name,
                                resource,
                            ));
                        }
                    }

                    for (_, resource) in needs.iter() {
                        if config.find_possession(resource).is_err() {
                            return Err(format!(
                                "Crafting advancement {:?} for plant {:?} uses unknown resource {:?} in recipe for {:?}",
                                adv.title,
                                arch.name,
                                resource,
                                makes
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
