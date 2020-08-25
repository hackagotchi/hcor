use flate2::{write::DeflateEncoder, Compression};
use hcor::wormhole::{Ask::*, AskedNote::*, ItemAsk::*, PlantAsk::*};
use hcor::{config, item, plant, Item, ItemId, Plant, SteaderId, Tile, TileId};
use serde::Serialize;
use std::{
    fmt,
    time::{Duration, Instant},
};
use uuid::Uuid;

fn deflate_bincode_len<S: Serialize>(s: &S, lvl: u32) -> Result<usize, String> {
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::new(lvl));

    bincode::serialize_into(&mut enc, s)
        .map_err(|e| format!("couldn't transpile bincode: {}", e))?;

    enc.finish()
        .map_err(|e| format!("couldn't deflate bincode: {}", e))
        .map(|b| b.len())
}

fn zstd_bincode_len<S: Serialize>(s: &S, lvl: i32) -> Result<usize, String> {
    zstd::encode_all(
        bincode::serialize(&s)
            .map_err(|e| format!("couldn't transpile bincode: {}", e))?
            .as_slice(),
        lvl,
    )
    .map_err(|e| format!("couldn't zstd bincode: {}", e))
    .map(|b| b.len())
}

fn bincode_len<S: Serialize>(s: &S) -> Result<usize, String> {
    bincode::serialize(s)
        .map_err(|e| format!("couldn't transpile bincode: {}", e))
        .map(|b| b.len())
}

fn json_bytes_len<S: Serialize>(s: &S) -> Result<usize, String> {
    serde_json::to_vec(s)
        .map_err(|e| format!("couldn't transpile json: {}", e))
        .map(|b| b.len())
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Source {
    RawBincode,
    RawJson,
    CompressedBincode(u32),
    CompressedZstd(i32),
}
impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Source::RawJson => write!(f, "Json"),
            Source::RawBincode => write!(f, "Bincode, no compression"),
            Source::CompressedBincode(i) => write!(f, "Bincode, compression level {}", i),
            Source::CompressedZstd(i) => write!(f, "Zstd, compression level {}", i),
        }
    }
}

struct Entry {
    source: Source,
    bytes: usize,
    elapsed: Duration,
}
impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, bytes size: {}, time elapsed: {:#?}",
            self.source, self.bytes, self.elapsed
        )
    }
}

fn time_each_encoding<S: Serialize>(s: &S) -> Vec<Entry> {
    let mut results: Vec<Entry> = Default::default();

    fn time(source: Source, f: impl Fn() -> Result<usize, String>) -> Entry {
        let start = Instant::now();
        let mut bytes = 0;
        for _ in 0..1000 {
            bytes = f().expect("failed while timing");
        }
        let elapsed = Instant::now().duration_since(start) / 1000;
        Entry {
            source,
            bytes,
            elapsed,
        }
    }

    results.push(time(Source::RawJson, || json_bytes_len(s)));
    results.push(time(Source::RawBincode, || bincode_len(s)));

    for i in 0..10 {
        results.push(time(Source::CompressedBincode(i), || {
            deflate_bincode_len(s, i)
        }));
    }

    for i in 0..10 {
        results.push(time(Source::CompressedZstd(i), || zstd_bincode_len(s, i)));
    }

    results
}

fn test_each<S: Serialize + fmt::Debug>(all: Vec<S>) {
    println!("serializing list of all:");
    for entry in time_each_encoding(&all) {
        println!("{}", entry);
    }

    for one in &all {
        println!("serializing just: {:#?}", one);
        for entry in time_each_encoding(one) {
            println!("{}", entry);
        }
    }
}

fn main() {
    let t_rnd = || TileId(Uuid::new_v4());
    let i_rnd = || ItemId(Uuid::new_v4());
    let s_rnd = || SteaderId(Uuid::new_v4());

    let i_conf = || *hcor::CONFIG.items.keys().next().unwrap();

    let plant = || {
        Plant::from_conf(
            s_rnd(),
            t_rnd(),
            *hcor::CONFIG.plants.keys().next().unwrap(),
        )
    };
    let rub_effect = || plant::RubEffect {
        effect_id: plant::RubEffectId(Uuid::new_v4()),
        item_conf: i_conf(),
        effect_index: 0,
    };
    let item = || Item::from_conf(i_conf(), s_rnd(), item::Acquisition::spawned());

    test_each(vec![
        hcor::Ask::KnowledgeSnort { xp: 100 },
        Plant(Summon {
            tile_id: t_rnd(),
            seed_item_id: i_rnd(),
        }),
        Plant(Slaughter { tile_id: t_rnd() }),
        Plant(Craft {
            tile_id: t_rnd(),
            recipe_index: 0,
        }),
        Plant(Rub {
            tile_id: t_rnd(),
            rub_item_id: i_rnd(),
        }),
        Item(Spawn {
            item_conf: i_conf(),
            amount: 10,
        }),
        Item(Throw {
            receiver_id: s_rnd(),
            item_ids: (0..20).map(|_| i_rnd()).collect(),
        }),
        Item(Hatch {
            hatchable_item_id: i_rnd(),
        }),
        TileSummon {
            tile_redeemable_item_id: i_rnd(),
        },
    ]);
    test_each(vec![
        KnowledgeSnortResult(Ok(10)),
        PlantSummonResult(Ok(plant())),
        PlantSlaughterResult(Ok(plant())),
        PlantCraftStartResult(Ok(plant::Craft {
            recipe_conf: hcor::CONFIG.recipes().next().unwrap().conf,
        })),
        PlantRubStartResult(Ok((0..3).map(|_| rub_effect()).collect())),
        TileSummonResult(Ok(Tile::new(s_rnd()))),
        ItemSpawnResult(Ok((0..4).map(|_| item()).collect())),
        ItemThrowResult(Ok((0..100).map(|_| item()).collect())),
        ItemHatchResult(Ok(config::evalput::Output {
            items: (0..20).map(|_| item()).collect(),
            xp: 100,
        })),
    ])
}
