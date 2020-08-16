use serde::{Serialize, Deserialize};
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompiledOutput<I: Clone> {
    xp: usize,
    items: Vec<I>,
}
impl<I: Clone> CompiledOutput<I> {
    fn new() -> Self {
        Self {
            xp: 0,
            items: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexOutput<I: Clone> {
    All(Vec<ComplexOutput<I>>),
    OneOf(Vec<(f32, ComplexOutput<I>)>),
    Times(usize, Box<ComplexOutput<I>>),
    Chance(f32, Box<ComplexOutput<I>>),
    Xp(usize),
    Item(I),
}

impl<I: Clone> ComplexOutput<I> {
    pub fn compiled(self) -> CompiledOutput<I> {
        let mut compiled = CompiledOutput::new();
        self.compile(&mut compiled);
        compiled
    }

    fn map_item<T: Clone>(self, map: &mut impl FnMut(I) -> T) -> ComplexOutput<T> {
        use ComplexOutput::*;

        match self {
            All(these) => All(these.into_iter().map(|i| i.map_item(map)).collect()),
            OneOf(these) => OneOf(these.into_iter().map(|(c, i)| (c, i.map_item(map))).collect()),
            Times(m, body) => Times(m, Box::new(body.map_item(map))),
            Chance(c, body) => Chance(c, Box::new(body.map_item(map))),
            Xp(xp) => Xp(xp),
            Item(i) => Item(map(i)),
        }
    }

    fn compile(self, compiled: &mut CompiledOutput<I>) {
        use ComplexOutput::*;

        match self {
            All(these) => {
                for x in these {
                    x.compile(compiled)
                }
            },
            OneOf(these) => {
                let mut r: f32 = rand::thread_rng().gen_range(0.0, 1.0);
                for (chance, x) in these {
                    r -= chance;
                    if r < 0.0 {
                        x.compile(compiled);
                        break;
                    }
                }
            },
            Times(times, body) => {
                for _ in 0..times {
                    body.clone().compile(compiled)
                }
            }
            Chance(chance, body) => {
                if rand::thread_rng().gen_range(0.0, 1.0) < chance {
                    body.compile(compiled)
                }
            }
            Xp(amount) => compiled.xp += amount,
            Item(s) => compiled.items.push(s),
        }
    }
}

#[test]
fn test() {
    let raw: ComplexOutput<String> = serde_yaml::from_str(r#"
All:
  - Times: [10, OneOf: [
        [0.15, Item: Cupcake],
        [0.15, Item: Strudel],
        [0.70, Xp: 100],
    ]]
  - Xp: 500
  - Chance: [0.8, All: [
        Item: Lollipop,
        Chance: [0.7, All: [
            Item: Ice Cream,
            Times: [4, Item: Cupcake],
        ]]
    ]]
    "#).unwrap();
    let compiled = raw.clone().compiled();

    println!("{:#?}", compiled);
}
