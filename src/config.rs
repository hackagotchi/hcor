use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum ConfigError {
    UnknownArchetypeName(String),
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ConfigError::*;
        match self {
            UnknownArchetypeName(name) => write!(f, "no archetype by the name of {:?}", name),
        }
    }
}

pub type LootTableHandle = usize;
pub type LootTable = Vec<(SpawnRate, String)>;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub special_users: Vec<String>,
    pub profile_archetype: ProfileArchetype,
    pub plant_archetypes: Vec<PlantArchetype>,
    pub possession_archetypes: Vec<Archetype>,
}

pub fn spawn<'rng, Handle: Clone>(
    table: &'rng Vec<(SpawnRate, Handle)>,
    rng: &'rng mut impl rand::RngCore
) -> impl Iterator<Item=Handle> + 'rng {
    table
        .iter()
        .flat_map(move |(spawn_rate, h)| {
            (0..spawn_rate.gen_count(rng)).map(move |_| {
                h.clone()
            })
        })
}

/// Quite similar to spawn(), but it also returns a list of the
/// leading spawn rates for the rows in the table the spawn qualified for.
pub fn spawn_with_percentile<Handle: Clone>(
    table: &Vec<(SpawnRate, Handle)>,
    rng: &mut impl rand::RngCore
) -> (Vec<Handle>, f32) {
    let (handles, rows): (Vec<Vec<Handle>>, Vec<f32>) = table
        .iter()
        .filter_map(|(spawn_rate, h)| {
            let count = spawn_rate.gen_count(rng);

            if count == 0 {
                None
            } else {
                Some((
                    (0..count)
                        .map(move |_| h.clone())
                        .collect::<Vec<_>>(),
                    spawn_rate.0
                ))
            }
        })
        .unzip();

    (
        handles
            .into_iter()
            .flat_map(|h| h.into_iter())
            .collect(),
        {
            let best_roll: f32 = table.iter().map(|(SpawnRate(c, _), _)| c).copied().product();
            1.0 - ((rows.into_iter().product::<f32>() - best_roll) / (1.0 - best_roll))
        }
    )
}

impl Config {
    #[allow(dead_code)]
    pub fn find_plant<S: AsRef<str>>(&self, name: &S) -> Result<&PlantArchetype, ConfigError> {
        self.plant_archetypes
            .iter()
            .find(|x| name.as_ref() == x.name)
            .ok_or(ConfigError::UnknownArchetypeName(name.as_ref().to_string()))
    }
    pub fn find_plant_handle<S: AsRef<str>>(
        &self,
        name: &S,
    ) -> Result<ArchetypeHandle, ConfigError> {
        self.plant_archetypes
            .iter()
            .position(|x| name.as_ref() == x.name)
            .ok_or(ConfigError::UnknownArchetypeName(name.as_ref().to_string()))
    }
    #[allow(dead_code)]
    pub fn find_possession<S: AsRef<str>>(&self, name: &S) -> Result<&Archetype, ConfigError> {
        self.possession_archetypes
            .iter()
            .find(|x| name.as_ref() == x.name)
            .ok_or(ConfigError::UnknownArchetypeName(name.as_ref().to_string()))
    }
    pub fn find_possession_handle<S: AsRef<str>>(
        &self,
        name: &S,
    ) -> Result<ArchetypeHandle, ConfigError> {
        self.possession_archetypes
            .iter()
            .position(|x| name.as_ref() == x.name)
            .ok_or(ConfigError::UnknownArchetypeName(name.as_ref().to_string()))
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileArchetype {
    pub advancements: AdvancementSet<HacksteadAdvancementSum>,
}

pub type HacksteadAdvancement = Advancement<HacksteadAdvancementSum>;
pub type HacksteadAdvancementSet = AdvancementSet<HacksteadAdvancementSum>;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HacksteadAdvancementKind {
    Land { pieces: u32 },
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HacksteadAdvancementSum {
    pub land: u32,
    pub xp: u64,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GotchiArchetype {
    pub base_happiness: u64,
    #[serde(default)]
    pub plant_effects: Vec<(String, PlantAdvancement)>,
    pub hatch_table: Option<LootTable>,
    #[serde(default)]
    pub welcome_gift: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeedArchetype {
    pub grows_into: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LandUnlock {
    pub requires_xp: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemApplication {
    short_description: String,
    advancements: Vec<ItemApplicationAdvancement>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemApplicationAdvancement {
    duration: f32,
    effect: PlantAdvancement,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeepsakeArchetype {
    pub unlocks_land: Option<LandUnlock>,
    #[serde(default)]
    pub plant_effects: Vec<(String, PlantAdvancement)>,
    #[serde(default)]
    pub item_application: Option<ItemApplication>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ArchetypeKind {
    Gotchi(GotchiArchetype),
    Seed(SeedArchetype),
    Keepsake(KeepsakeArchetype),
}
impl ArchetypeKind {
    pub fn category(&self) -> crate::Category {
        use crate::Category;
        match self {
            ArchetypeKind::Gotchi(_) => Category::Gotchi,
            _ => Category::Misc,
        }
    }
    pub fn keepsake(&self) -> Option<&KeepsakeArchetype> {
        match self {
            ArchetypeKind::Keepsake(k) => Some(k),
            _ => None,
        }
    }
    pub fn gotchi(&self) -> Option<&GotchiArchetype> {
        match self {
            ArchetypeKind::Gotchi(g) => Some(g),
            _ => None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Archetype {
    pub name: String,
    pub description: String,
    pub kind: ArchetypeKind,
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
    OneOf(Vec<(f32, Handle)>),
    AllOf(Vec<(usize, Handle)>),
}
impl<Handle: Clone> RecipeMakes<Handle> {
    /// Returns one possible output, randomly (but properly weighted)
    /// if more than one is possible.
    pub fn any(&self) -> Handle {
        use rand::Rng;
        use RecipeMakes::*;

        match self {
            Just(_, h) => h.clone(),
            OneOf(these) => {
                let mut x: f32 = rand::thread_rng().gen_range(0.0, 1.0);
                these
                    .iter()
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
            AllOf(these) => {
                let total = these.iter().map(|(count, _)| *count).sum::<usize>() as f32;
                OneOf(
                    these
                        .iter()
                        .map(|(count, h)| (*count as f32 / total, h.clone()))
                        .collect()
                )
                .any()
            }
        }
    }

    /// A list of everything that could possibly come from this recipe
    pub fn all(&self) -> Vec<(Handle, usize)> {
        use RecipeMakes::*;

        match self {
            Just(_, h) => [(h.clone(), 1)].iter().cloned().collect(),
            OneOf(these) => {
                these
                    .iter()
                    .map(|(_, h)| (h.clone(), 1))
                    .collect()
            }
            AllOf(these) => {
                these
                    .iter()
                    .map(|(count, h)| (h.clone(), *count))
                    .collect()
            }
        }
    }

    /// A proper output given the constraints of this recipe;
    /// for example, OneOf will always return one of the possibilities,
    /// properly weighted.
    pub fn output(self) -> Vec<Handle> {
        use RecipeMakes::*;

        match &self {
            Just(_, _) | OneOf(_) => vec![self.any()],
            AllOf(_) => self
                .all()
                .into_iter()
                .flat_map(|(what, count)| (0..count).map(move |_| what.clone()))
                .collect(),
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
                    .map(|(chance, what)| {
                        format!(
                            "a {} _{}_ (*{:.2}%* chance)",
                            emojify(&what.name),
                            what.name,
                            chance * 100.0
                        )
                    })
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
        }
    }
}

impl RecipeMakes<ArchetypeHandle> {
    pub fn lookup_handles(self) -> Option<RecipeMakes<&'static Archetype>> {
        use RecipeMakes::*;

        fn lookup(ah: ArchetypeHandle) -> Option<&'static Archetype> {
            CONFIG.possession_archetypes.get(ah)
        }

        Some(match self {
            Just(n, ah) => Just(n, lookup(ah)?),
            OneOf(l) => OneOf(
                l.into_iter()
                    .map(|(c, ah)| Some((c, lookup(ah)?)))
                    .collect::<Option<_>>()?,
            ),
            AllOf(l) => AllOf(
                l.into_iter()
                    .map(|(c, ah)| Some((c, lookup(ah)?)))
                    .collect::<Option<_>>()?,
            ),
        })
    }
}

impl RecipeMakes<String> {
    pub fn find_handles(self) -> Result<RecipeMakes<ArchetypeHandle>, ConfigError> {
        use RecipeMakes::*;

        fn find(name: String) -> Result<ArchetypeHandle, ConfigError> {
            CONFIG.find_possession_handle(&name)
        }

        Ok(match self {
            Just(n, ah) => Just(n, find(ah)?),
            OneOf(l) => OneOf(
                l.into_iter()
                    .map(|(c, ah)| Ok((c, find(ah)?)))
                    .collect::<Result<_, _>>()?,
            ),
            AllOf(l) => AllOf(
                l.into_iter()
                    .map(|(c, ah)| Ok((c, find(ah)?)))
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
}

/// Recipe is generic over the way Archetypes are referred to
/// to make it easy to use Strings in the configs and ArchetypeHandles
/// at runtime
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Recipe<Handle: Clone> {
    pub needs: Vec<(usize, Handle)>,
    pub makes: RecipeMakes<Handle>,
    #[serde(default)]
    pub destroys_plant: bool,
    pub time: f32,
}
impl Recipe<ArchetypeHandle> {
    pub fn satisfies(&self, inv: &[crate::Possession]) -> bool {
        self.needs.iter().copied().all(|(count, ah)| {
            let has = inv.iter().filter(|x| x.archetype_handle == ah).count();
            count <= has
        })
    }
    pub fn lookup_handles(self) -> Option<Recipe<&'static Archetype>> {
        Some(Recipe {
            makes: self.makes.lookup_handles()?,
            needs: self
                .needs
                .into_iter()
                .map(|(n, x)| Some((n, CONFIG.possession_archetypes.get(x)?)))
                .collect::<Option<Vec<(_, &Archetype)>>>()?,
            time: self.time,
            destroys_plant: self.destroys_plant,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct SpawnRate(pub f32, pub (f32, f32));
impl SpawnRate {
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

/// A leading chance, then a min and max percent of things returned.
/// Should be reminiscent of loot tables.
pub type CraftReturnChance = (f32, (f32, f32));
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlantAdvancementKind {
    Neighbor(Box<PlantAdvancementKind>),
    /// Stores the number of extra cycles to add for the duration of the effect
    TimeAcceleration(f32),
    Xp(f32),
    YieldSpeed(f32),
    YieldSize(f32),
    Yield(Vec<(SpawnRate, String)>),
    Craft(Vec<Recipe<String>>),
    CraftReturn(CraftReturnChance),
    DoubleCraftYield(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(bound(deserialize = ""))]
pub struct PlantAdvancementSum {
    // xp
    pub xp: u64,
    pub xp_multiplier: u64,
    // yield
    pub yield_speed_multiplier: f32,
    pub yield_size_multiplier: f32,
    pub yields: Vec<(SpawnRate, ArchetypeHandle)>,
    // craft
    pub double_craft_yield_chance: f32,
    pub craft_return_chances: Vec<CraftReturnChance>,
    pub recipes: Vec<Recipe<ArchetypeHandle>>,
}
impl AdvancementSum for PlantAdvancementSum {
    type Kind = PlantAdvancementKind;

    fn new(unlocked: &[&Advancement<Self>]) -> Self {
        use PlantAdvancementKind::*;

        let mut time_acceleration: f32 = 1.0;
        // xp
        let mut xp = 0;
        let mut xp_multiplier = 1;
        // yield
        let mut yield_speed_multiplier = 1.0;
        let mut yield_size_multiplier = 1.0;
        let mut yields = vec![];
        // craft
        let mut double_craft_yield_chance = 0.0;
        let mut craft_return_chances = vec![];
        let mut recipes = vec![];

        for k in unlocked.iter() {
            xp += k.xp;

            // apply neighbor upgrades as if they weren't neighbor upgrades :D
            let kind = match &k.kind {
                Neighbor(n) => &**n,
                other => other,
            };

            match kind {
                Xp(xp_gain) => xp_multiplier += *xp_gain as u64,
                TimeAcceleration(extra) => time_acceleration += extra,
                YieldSpeed(multiplier) => yield_speed_multiplier *= multiplier,
                Neighbor(..) => {}
                YieldSize(multiplier) => yield_size_multiplier *= multiplier,
                CraftReturn(chance) => craft_return_chances.push(*chance),
                DoubleCraftYield(chance) => double_craft_yield_chance += chance,
                Yield(resources) => yields.append(
                    &mut resources
                        .iter()
                        .map(|(c, s)| Ok((*c, CONFIG.find_possession_handle(s)?)))
                        .collect::<Result<Vec<_>, ConfigError>>()
                        .expect("couldn't find archetype for advancement yield"),
                ),
                Craft(new_recipes) => recipes.append(
                    &mut new_recipes
                        .iter()
                        .map(|r| {
                            Ok(Recipe {
                                makes: r.makes.clone().find_handles()?,
                                needs: r
                                    .needs
                                    .iter()
                                    .map(|(c, s)| Ok((*c, CONFIG.find_possession_handle(s)?)))
                                    .collect::<Result<Vec<_>, ConfigError>>()?,

                                time: r.time,
                                destroys_plant: r.destroys_plant,
                            })
                        })
                        .collect::<Result<Vec<_>, ConfigError>>()
                        .expect("couldn't find archetype for crafting advancement"),
                ),
            }
        }

        yields = yields
            .into_iter()
            .map(|(SpawnRate(guard, (lo, hi)), name)| {
                (
                    SpawnRate(
                        (guard * yield_size_multiplier).min(1.0),
                        (lo * yield_size_multiplier, hi * yield_size_multiplier),
                    ),
                    name,
                )
            })
            .collect();

        Self {
            // xp
            xp,
            xp_multiplier,
            // yield
            yield_speed_multiplier,
            yield_size_multiplier,
            yields,
            // craft
            double_craft_yield_chance,
            craft_return_chances,
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
    pub xp: u64,
    pub art: String,
    pub title: String,
    pub description: String,
    pub achiever_title: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub fn unlocked(&self, xp: u64) -> impl Iterator<Item = &Advancement<S>> {
        std::iter::once(&self.base).chain(self.rest.iter().take(self.current_position(xp)))
    }

    pub fn get(&self, index: usize) -> Option<&Advancement<S>> {
        if index == 0 {
            Some(&self.base)
        } else {
            self.rest.get(index - 1)
        }
    }

    pub fn increase_xp(&self, xp: &mut u64, amt: u64) -> Option<&Advancement<S>> {
        *xp += amt;
        self.next(xp.checked_sub(1).unwrap_or(0))
            .filter(|&x| self.next(*xp).map(|n| *x != *n).unwrap_or(false))
    }

    pub fn sum<'a>(
        &'a self,
        xp: u64,
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

    pub fn raw_sum(&self, xp: u64) -> S {
        S::new(&self.unlocked(xp).collect::<Vec<_>>())
    }

    pub fn max<'a>(&'a self, extra_advancements: impl Iterator<Item = &'a Advancement<S>>) -> S {
        S::new(
            &self
                .all()
                .filter(|&x| S::filter_base(x))
                .chain(extra_advancements)
                .collect::<Vec<_>>(),
        )
    }

    pub fn current(&self, xp: u64) -> &Advancement<S> {
        self.get(self.current_position(xp)).unwrap_or(&self.base)
    }

    pub fn next(&self, xp: u64) -> Option<&Advancement<S>> {
        self.get(self.current_position(xp) + 1)
    }

    pub fn current_position(&self, xp: u64) -> usize {
        let mut state = 0;
        self.all()
            .position(|x| {
                state += x.xp;
                state > xp
            })
            .unwrap_or(self.rest.len() + 1)
            .checked_sub(1)
            .unwrap_or(0)
    }
}

#[test]
fn upgrade_increase() {
    for arch in CONFIG.plant_archetypes.iter() {
        let adv = &arch.advancements;
        let last = adv.rest.last().unwrap();
        for xp in 0..last.xp {
            assert!(
                adv.current(xp).xp <= xp,
                "when xp is {} for {} the current advancement has more xp({})",
                xp,
                arch.name,
                adv.current(xp).xp
            );
        }
    }
}

/// In the config, you can specify the names of archetypes.
/// If you're Rishi, you might spell one of those names wrong.
/// This function helps you make sure you didn't do that.
pub fn check_archetype_name_matches(config: &Config) -> Result<(), String> {
    for a in config.possession_archetypes.iter() {
        match &a.kind {
            ArchetypeKind::Seed(sa) => if config.find_plant(&sa.grows_into).is_err() {
                return Err(format!(
                    "seed archetype {:?} claims it grows into unknown plant archetype {:?}",
                    a.name,
                    sa.grows_into,
                ))
            },
            ArchetypeKind::Gotchi(ga) => {
                if let Some(table) = &ga.hatch_table {
                    for (_, spawn) in table.iter() {
                        if config.find_possession(spawn).is_err() {
                            return Err(format!(
                                "gotchi archetype {:?} claims it hatches into unknown possession archetype {:?}",
                                a.name,
                                spawn,
                            ))
                        }
                    }
                }
                for (plant, _) in &ga.plant_effects {
                    if config.find_plant(plant).is_err() {
                        return Err(format!(
                            "gotchi archetype {:?} claims it affects unknown plant archetype {:?}",
                            a.name,
                            plant,
                        ))
                    }
                }
            }
            _ => {}
        }
    }

    for (arch, adv) in config.plant_archetypes
        .iter()
        .flat_map(|arch| {
            arch
                .advancements
                .all()
                .map(move |adv| (arch.clone(), adv))
        })
    {
        use PlantAdvancementKind::*;

        match &adv.kind {
            Yield(resources) => {
                for (_, item_name) in resources.iter() {
                    if config.find_possession(item_name).is_err() {
                        return Err(format!(
                            "Yield advancement {:?} for plant {:?} includes unknown resource {:?}",
                            adv.title,
                            arch.name,
                            item_name,
                        ))
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
                            ))
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
                            ))
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[test]
fn archetype_name_matches() {
    check_archetype_name_matches(&*CONFIG)
        .unwrap_or_else(|e| panic!("{}", e));
}
