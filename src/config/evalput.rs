use crate::item;
use rand::Rng;
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output<I: Clone> {
    xp: usize,
    items: Vec<I>,
}
impl<I: Clone> Output<I> {
    fn new() -> Self {
        Self {
            xp: 0,
            items: vec![],
        }
    }
}

pub type RawEvalput = Evalput<String>;

impl super::Verify for RawEvalput {
    type Verified = Evalput<item::Conf>;

    fn verify_raw(self, raw: &super::RawConfig) -> super::VerifResult<Self::Verified> {
        self.ok_or_item(&mut |item_name| raw.item_conf(&item_name))
    }

    fn context(&self) -> Option<String> {
        use Evalput::*;
        Some(format!(
            "in an evalput's {} node",
            match self {
                All(_) => "All",
                OneOf(_) => "OneOf",
                Amount(_, _) => "Amount",
                Chance(_, _) => "Chance",
                Xp(_) => "Xp",
                Item(_) => "Item",
            }
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Evalput<I: Clone> {
    All(Vec<Evalput<I>>),
    OneOf(Vec<(f32, Evalput<I>)>),
    Amount(
        #[serde(deserialize_with = "num_or_variant")] Repeats,
        Box<Evalput<I>>,
    ),
    Chance(f32, Box<Evalput<I>>),
    Xp(#[serde(deserialize_with = "num_or_variant")] Repeats),
    Item(I),
}

impl<I: Clone> Evalput<I> {
    pub fn evaluated(self, rng: &mut impl Rng) -> Output<I> {
        let mut output = Output::new();
        self.eval(&mut output, rng);
        output
    }

    pub fn map_item<T: Clone>(self, map: &mut impl FnMut(I) -> T) -> Evalput<T> {
        use Evalput::*;

        match self {
            All(these) => All(these.into_iter().map(|i| i.map_item(map)).collect()),
            OneOf(these) => OneOf(
                these
                    .into_iter()
                    .map(|(c, i)| (c, i.map_item(map)))
                    .collect(),
            ),
            Amount(m, body) => Amount(m, Box::new(body.map_item(map))),
            Chance(c, body) => Chance(c, Box::new(body.map_item(map))),
            Xp(xp) => Xp(xp),
            Item(i) => Item(map(i)),
        }
    }

    pub fn ok_or_item<T: Clone, E>(
        self,
        ok_or: &mut impl FnMut(I) -> Result<T, E>,
    ) -> Result<Evalput<T>, E> {
        use Evalput::*;

        Ok(match self {
            All(these) => All(these
                .into_iter()
                .map(|i| i.ok_or_item(ok_or))
                .collect::<Result<_, E>>()?),
            OneOf(these) => OneOf(
                these
                    .into_iter()
                    .map(|(c, i)| Ok((c, i.ok_or_item(ok_or)?)))
                    .collect::<Result<_, E>>()?,
            ),
            Amount(m, body) => Amount(m, Box::new(body.ok_or_item(ok_or)?)),
            Chance(c, body) => Chance(c, Box::new(body.ok_or_item(ok_or)?)),
            Xp(xp) => Xp(xp),
            Item(i) => Item(ok_or(i)?),
        })
    }

    pub fn eval(&self, output: &mut Output<I>, rng: &mut impl Rng) {
        use Evalput::*;

        match self {
            All(these) => {
                for x in these {
                    x.eval(output, rng)
                }
            }
            OneOf(these) => {
                let mut r: f32 = rng.gen_range(0.0, 1.0);
                for (chance, x) in these {
                    r -= chance;
                    if r < 0.0 {
                        x.eval(output, rng);
                        break;
                    }
                }
            }
            Amount(times, body) => {
                for _ in 0..times.eval(rng) {
                    body.eval(output, rng)
                }
            }
            Chance(chance, body) => {
                if rng.gen_range(0.0, 1.0) < *chance {
                    body.eval(output, rng)
                }
            }
            Xp(amount) => output.xp += amount.eval(rng),
            Item(s) => output.items.push(s.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Repeats {
    Just(f32),
    Between(f32, f32),
}
impl Repeats {
    pub fn eval(&self, rng: &mut impl Rng) -> usize {
        let x = match self {
            &Repeats::Just(u) => u,
            &Repeats::Between(lo, hi) => rng.gen_range(lo, hi),
        };
        let remaining_decimal = x - x.floor();
        let extra = remaining_decimal < rng.gen_range(0.0, 1.0);
        x.floor() as usize + extra as usize
    }
}
fn num_or_variant<'de, D>(deserializer: D) -> Result<Repeats, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use de::value::{MapAccessDeserializer, SeqAccessDeserializer};
    use Repeats::*;

    struct NumOrVariant;
    impl<'de> Visitor<'de> for NumOrVariant {
        type Value = Repeats;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "`n` OR `[n, n]` OR `Just: n` OR `Between: [n, n]` where `n` is any positive number"
            )
        }

        #[inline]
        fn visit_u64<E>(self, value: u64) -> Result<Repeats, E> {
            Ok(Just(value as f32))
        }

        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Repeats, E> {
            Ok(Just(value as f32))
        }

        fn visit_seq<M>(self, seq: M) -> Result<Repeats, M::Error>
        where
            M: SeqAccess<'de>,
        {
            let (lo, hi) = Deserialize::deserialize(SeqAccessDeserializer::new(seq))?;
            Ok(Between(lo, hi))
        }

        fn visit_map<M>(self, map: M) -> Result<Repeats, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_any(NumOrVariant)
}

#[test]
fn test_repeats_deserialize() {
    let mut rng = rand::thread_rng();
    let output = serde_yaml::from_str::<Evalput<String>>(
        r#"
All:
    - Amount: [ 1, Item: Bag]
    - Amount: [ Just: 1, Item: Bag]
    - Amount: [ Between: [1, 2], Item: Bag]
    "#,
    )
    .unwrap()
    .evaluated(&mut rng);

    assert!(output.items.len() >= 3)
}

#[test]
fn test_serialize_deeply_nested() {
    let raw: Evalput<String> = serde_yaml::from_str(
        r#"
All:
  - Amount: [10, OneOf: [
        [0.15, Item: Cupcake],
        [0.15, Item: Strudel],
        [0.70, Xp: 100],
    ]]
  - Xp: 500
  - Chance: [0.8, All: [
        Item: Lollipop,
        Chance: [0.7, All: [
            Item: Ice Cream,
            Amount: [ Between: [4, 6], Item: Cupcake],
        ]]
    ]]
    "#,
    )
    .unwrap();
    let mut rng = rand::thread_rng();
    let output = raw.clone().evaluated(&mut rng);

    println!("{:#?}", output);
}
