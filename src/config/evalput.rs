use rand::Rng;
#[cfg(feature = "config_verify")]
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
#[cfg(feature = "config_verify")]
use std::fmt;

#[cfg(feature = "config_verify")]
use super::{VerifError, VerifResult};
#[cfg(feature = "config_verify")]
use crate::item;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OneOfRow<I: Clone>(
    #[cfg_attr(feature = "config_verify", serde(deserialize_with = "one_of_chance_parse"))] OneOfChance,
    Evalput<I>,
);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Evalput<I: Clone> {
    All(Vec<Evalput<I>>),
    OneOf(Vec<OneOfRow<I>>),
    Amount(
        #[cfg_attr(feature = "config_verify", serde(deserialize_with = "repeats_parse"))] Repeats,
        Box<Evalput<I>>,
    ),
    Chance(f64, Box<Evalput<I>>),
    Xp(#[cfg_attr(feature = "config_verify", serde(deserialize_with = "repeats_parse"))] Repeats),
    Item(I),
    Nothing,
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
                    .map(|OneOfRow(c, i)| OneOfRow(c, i.map_item(map)))
                    .collect(),
            ),
            Amount(m, body) => Amount(m, Box::new(body.map_item(map))),
            Chance(c, body) => Chance(c, Box::new(body.map_item(map))),
            Xp(xp) => Xp(xp),
            Item(i) => Item(map(i)),
            Nothing => Nothing,
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
                    .map(|OneOfRow(c, i)| Ok(OneOfRow(c, i.ok_or_item(ok_or)?)))
                    .collect::<Result<_, E>>()?,
            ),
            Amount(m, body) => Amount(m, Box::new(body.ok_or_item(ok_or)?)),
            Chance(c, body) => Chance(c, Box::new(body.ok_or_item(ok_or)?)),
            Xp(xp) => Xp(xp),
            Item(i) => Item(ok_or(i)?),
            Nothing => Nothing,
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
                let mut r: f64 = rng.gen_range(0.0, 1.0);
                for OneOfRow(chance, x) in these {
                    r -= chance.chance().unwrap_or(0.0);
                    if r < 0.0 || chance.is_rest() {
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
            Nothing => {}
        }
    }
}

#[cfg(feature = "config_verify")]
pub type RawEvalput = Evalput<String>;

#[cfg(feature = "config_verify")]
impl super::Verify for RawEvalput {
    type Verified = Evalput<item::Conf>;

    fn verify_raw(self, raw: &super::RawConfig) -> VerifResult<Self::Verified> {
        use Evalput::*;

        fn err(e: impl AsRef<str>) -> VerifResult<()> {
            Err(VerifError::custom(e))
        }

        fn verify_repeats(rpts: &Repeats) -> VerifResult<()> {
            use Repeats::*;

            if let Just(x) = rpts {
                if *x == 1.0 {
                    err("There is no point in repeating something just once.")?;
                }
            }

            if let Between(hi, lo) = rpts {
                if hi == lo {
                    err("There is no point in having the upper and lower bounds be identical.")?;
                }
            }

            Ok(())
        }

        Ok(match self {
            All(these) => All(these
                .into_iter()
                .map(|i| i.verify(raw))
                .collect::<VerifResult<_>>()?),
            OneOf(these) => {
                let has_rest = these.iter().any(|OneOfRow(c, _)| c.is_rest());
                let adds_up_to = these
                    .iter()
                    .filter_map(|OneOfRow(c, _)| c.chance())
                    .sum::<f64>();

                if !(has_rest || adds_up_to == 1.0) {
                    err("OneOf chances must add up to 1.0 or contain Rest")?;
                }

                if has_rest && adds_up_to == 1.0 {
                    err("There is no point in having Rest when the other chances add up to 1.0")?;
                }

                if adds_up_to > 1.0 {
                    err("OneOf chances should not exceed 1.0")?;
                }

                if these.len() == 1 {
                    err("There is no point in having a OneOf with only one option.")?;
                }

                OneOf(
                    these
                        .into_iter()
                        .map(|OneOfRow(c, i)| Ok(OneOfRow(c, i.verify(raw)?)))
                        .collect::<VerifResult<_>>()?,
                )
            }
            Amount(m, body) => {
                verify_repeats(&m)?;
                Amount(m, Box::new(body.verify(raw)?))
            }
            Chance(c, body) => {
                if c == 1.0 {
                    err("There is no point in having a 100% chance like this.")?;
                }
                if c > 1.0 {
                    err("It's impossible to have a greater than 100% chance like this.")?;
                }

                Chance(c, Box::new(body.verify(raw)?))
            }
            Xp(xp) => {
                verify_repeats(&xp)?;
                Xp(xp)
            }
            Item(i) => Item(raw.item_conf(&i)?),
            Nothing => Nothing,
        })
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
                Nothing => "Nothing",
            }
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Repeats {
    Exactly(u64),
    Just(f64),
    Between(f64, f64),
}
impl Repeats {
    pub fn eval(&self, rng: &mut impl Rng) -> usize {
        let x = match *self {
            Repeats::Exactly(a) => return a as usize,
            Repeats::Just(u) => u,
            Repeats::Between(lo, hi) => rng.gen_range(lo, hi),
        };
        let remaining_decimal = x - x.floor();
        let extra = remaining_decimal < rng.gen_range(0.0, 1.0);
        x.floor() as usize + extra as usize
    }
}
#[cfg(feature = "config_verify")]
fn repeats_parse<'de, D>(deserializer: D) -> Result<Repeats, D::Error>
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
            Ok(Exactly(value))
        }

        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<Repeats, E> {
            Ok(Just(value))
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OneOfChance {
    Rest,
    Chance(f64),
}

impl OneOfChance {
    fn chance(self) -> Option<f64> {
        use OneOfChance::*;

        match self {
            Chance(f) => Some(f),
            Rest => None,
        }
    }

    fn is_rest(&self) -> bool {
        matches!(self, OneOfChance::Rest)
    }
}

#[cfg(feature = "config_verify")]
fn one_of_chance_parse<'de, D>(deserializer: D) -> Result<OneOfChance, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct NumOrVariant;
    impl<'de> Visitor<'de> for NumOrVariant {
        type Value = OneOfChance;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "`n` OR `Rest` where `n` is any positive decimal less than 1.0"
            )
        }

        #[inline]
        fn visit_f64<E>(self, value: f64) -> Result<OneOfChance, E>
        where
            E: de::Error,
        {
            if 0.0 < value && value < 1.0 {
                Ok(OneOfChance::Chance(value))
            } else {
                Err(de::Error::invalid_value(
                    de::Unexpected::Float(value),
                    &"a float between 0.0 and 1.0, exclusive",
                ))
            }
        }

        fn visit_str<E>(self, s: &str) -> Result<OneOfChance, E>
        where
            E: de::Error,
        {
            if s == "Rest" {
                Ok(OneOfChance::Rest)
            } else {
                Err(de::Error::invalid_value(
                    de::Unexpected::Str(s),
                    &"expected Rest",
                ))
            }
        }
    }
    deserializer.deserialize_any(NumOrVariant)
}

#[cfg(feature = "config_verify")]
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

#[cfg(feature = "config_verify")]
#[test]
fn test_serialize_deeply_nested() {
    let raw: Evalput<String> = serde_yaml::from_str(
        r#"
All:
  - Amount: [10, OneOf: [
        [0.15, Item: Cupcake],
        [0.15, Item: Strudel],
        [Rest, Xp: 100],
    ]]
  - Xp: 500
  - Chance: [0.8, All: [
        Item: Lollipop,
        Chance: [0.7, All: [
            Item: Ice Cream,
            Amount: [ [4, 6], Item: Cupcake],
        ]]
    ]]
    "#,
    )
    .unwrap();
    let mut rng = rand::thread_rng();
    let output = raw.clone().evaluated(&mut rng);

    println!("{:#?}", output);
}

#[cfg(feature = "config_verify")]
#[test]
fn test_one_of_verification() {
    use super::Verify;

    let raw = super::RawConfig {
        items: vec![super::FromFile::new(
            item::RawConfig {
                name: "pig".to_string(),
                description: "oink".to_string(),
                conf: item::Conf(uuid::Uuid::new_v4()),
                gotchi: None,
                grows_into: None,
                hatch_table: None,
                passive_plant_effects: vec![],
                plant_rub_effects: vec![],
                unlocks_land: None,
                welcome_gift: false,
            },
            "test".to_string(),
        )],
        ..Default::default()
    };

    let parse = |s| serde_yaml::from_str::<Evalput<String>>(s);
    assert!(parse(r#"OneOf: [ [0.1, Item: pig] ]"#)
        .unwrap()
        .verify(&raw)
        .is_err());
    assert!(parse(r#"OneOf: [ [0.1, Item: pig], [0.9, Item: pig], [1.1, Item: pig]]"#).is_err());
    assert!(parse(r#"OneOf: [ [1.0, Item: pig] ]"#).is_err());
    assert!(parse(r#"OneOf: [ [0.5, Item: pig], [0.5, Item: pig]]"#)
        .unwrap()
        .verify(&raw)
        .is_ok());
}
