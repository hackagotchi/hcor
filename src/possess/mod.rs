use crate::{config, market, CONFIG};
use config::{Archetype, ArchetypeHandle, ArchetypeKind};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod gotchi;
mod keepsake;
pub mod seed;

pub use gotchi::Gotchi;
pub use keepsake::Keepsake;
pub use seed::Seed;

pub trait Possessable: Sized {
    fn from_possession_kind(pk: PossessionKind) -> Option<Self>;
    fn into_possession_kind(self) -> PossessionKind;
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub enum PossessionKind {
    Gotchi(Gotchi),
    Seed(Seed),
    Keepsake(Keepsake),
}
impl PossessionKind {
    fn new(ah: ArchetypeHandle, owner_id: &str) -> Self {
        match CONFIG
            .possession_archetypes
            .get(ah)
            .unwrap_or_else(|| panic!("Unknown archetype: {}", ah))
            .kind
        {
            ArchetypeKind::Gotchi(_) => PossessionKind::Gotchi(Gotchi::new(ah, owner_id)),
            ArchetypeKind::Seed(_) => PossessionKind::Seed(Seed::new(ah, owner_id)),
            ArchetypeKind::Keepsake(_) => PossessionKind::Keepsake(Keepsake::new(ah, owner_id)),
        }
    }

    pub fn as_gotchi(self) -> Option<Gotchi> {
        match self {
            PossessionKind::Gotchi(g) => Some(g),
            _ => None,
        }
    }
    pub fn gotchi(&self) -> Option<&Gotchi> {
        match self {
            PossessionKind::Gotchi(g) => Some(g),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn is_gotchi(&self) -> bool {
        match self {
            PossessionKind::Gotchi(_) => true,
            _ => false,
        }
    }
    pub fn gotchi_mut(&mut self) -> Option<&mut Gotchi> {
        match self {
            PossessionKind::Gotchi(g) => Some(g),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_seed(self) -> Option<Seed> {
        match self {
            PossessionKind::Seed(g) => Some(g),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn seed(&self) -> Option<&Seed> {
        match self {
            PossessionKind::Seed(g) => Some(g),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn is_seed(&self) -> bool {
        match self {
            PossessionKind::Seed(_) => true,
            _ => false,
        }
    }
    #[allow(dead_code)]
    pub fn seed_mut(&mut self) -> Option<&mut Seed> {
        match self {
            PossessionKind::Seed(g) => Some(g),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_keepsake(self) -> Option<Keepsake> {
        match self {
            PossessionKind::Keepsake(g) => Some(g),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn keepsake(&self) -> Option<&Keepsake> {
        match self {
            PossessionKind::Keepsake(g) => Some(g),
            _ => None,
        }
    }
    #[allow(dead_code)]
    pub fn is_keepsake(&self) -> bool {
        match self {
            PossessionKind::Keepsake(_) => true,
            _ => false,
        }
    }
    #[allow(dead_code)]
    pub fn keepsake_mut(&mut self) -> Option<&mut Keepsake> {
        match self {
            PossessionKind::Keepsake(g) => Some(g),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Owner {
    pub id: String,
    pub acquisition: Acquisition,
}
impl Owner {
    pub fn farmer(id: String) -> Self {
        Self {
            id,
            acquisition: Acquisition::Farmed,
        }
    }
    pub fn crafter(id: String) -> Self {
        Self {
            id,
            acquisition: Acquisition::Crafted,
        }
    }
    pub fn hatcher(id: String) -> Self {
        Self {
            id,
            acquisition: Acquisition::Hatched,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Acquisition {
    Trade,
    Purchase { price: u64 },
    Farmed,
    Crafted,
    Hatched,
}
impl Acquisition {
    pub fn spawned() -> Self {
        Acquisition::Trade
    }
}
impl fmt::Display for Acquisition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Acquisition::Trade => write!(f, "Trade"),
            Acquisition::Farmed => write!(f, "Farmed"),
            Acquisition::Crafted => write!(f, "Crafted"),
            Acquisition::Hatched => write!(f, "Hatched"),
            Acquisition::Purchase { price } => write!(f, "Purchase({}gp)", price),
        }
    }
}

/// A copy of Possession for when you know what variant of PossessionKind
/// you have at compiletime and want to easily access its properties alongside
/// those properties all Possessions have.
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Possessed<P: Possessable> {
    pub inner: P,
    pub archetype_handle: ArchetypeHandle,
    pub id: uuid::Uuid,
    pub steader: String,
    pub ownership_log: Vec<Owner>,
    pub sale: Option<market::Sale>,
}
impl<P: Possessable> std::convert::TryFrom<Possession> for Possessed<P> {
    type Error = &'static str;

    fn try_from(p: Possession) -> Result<Self, Self::Error> {
        Possessed::from_possession(p).ok_or("wrongly typed possession")
    }
}
impl<P: Possessable> Possessed<P> {
    /// Use of the TryFrom implementation is preferred, but this
    /// static method is still exposed as a matter of convenience
    pub fn from_possession(p: Possession) -> Option<Possessed<P>> {
        let Possession {
            kind,
            archetype_handle,
            id,
            steader,
            ownership_log,
            sale,
        } = p;

        Some(Self {
            inner: P::from_possession_kind(kind)?,
            archetype_handle,
            id,
            steader,
            ownership_log,
            sale,
        })
    }
    pub fn into_possession(self) -> Possession {
        let Self {
            inner,
            archetype_handle,
            id,
            steader,
            ownership_log,
            sale,
        } = self;

        Possession {
            kind: P::into_possession_kind(inner),
            archetype_handle,
            id,
            steader,
            ownership_log,
            sale,
        }
    }
}

impl<P: Possessable> std::ops::Deref for Possessed<P> {
    type Target = Archetype;

    fn deref(&self) -> &Self::Target {
        self.archetype()
    }
}

impl<P: Possessable> Possessed<P> {
    pub fn archetype(&self) -> &Archetype {
        CONFIG
            .possession_archetypes
            .get(self.archetype_handle)
            .expect("invalid archetype handle")
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Possession {
    pub kind: PossessionKind,
    pub archetype_handle: ArchetypeHandle,
    pub id: uuid::Uuid,
    pub steader: String,
    pub ownership_log: Vec<Owner>,
    pub sale: Option<market::Sale>,
}

impl std::ops::Deref for Possession {
    type Target = Archetype;

    fn deref(&self) -> &Self::Target {
        self.archetype()
    }
}

impl Possession {
    pub fn new(archetype_handle: ArchetypeHandle, owner: Owner) -> Self {
        Self {
            kind: PossessionKind::new(archetype_handle, &owner.id),
            id: uuid::Uuid::new_v4(),
            archetype_handle,
            steader: owner.id.clone(),
            ownership_log: vec![owner],
            sale: None,
        }
    }

    pub fn nickname(&self) -> &str {
        match self.kind {
            PossessionKind::Gotchi(ref g) => &g.nickname,
            _ => &self.name,
        }
    }

    fn archetype(&self) -> &Archetype {
        CONFIG
            .possession_archetypes
            .get(self.archetype_handle)
            .expect("invalid archetype handle")
    }
}
