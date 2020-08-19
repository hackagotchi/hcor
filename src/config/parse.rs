use super::{FromFile, CONFIG_PATH};
use crate::{item, plant};
use ::log::*;
use serde::de::DeserializeOwned;
use serde_yaml::Value;
use std::{fmt, fs};

pub(super) fn read_items() -> Vec<FromFile<item::RawConfig>> {
    let mut items = vec![];

    for path in yml_files("items") {
        let pd = path.display();
        let file = fs::read_to_string(&path)
            .unwrap_or_else(|e| fatal!("\nCouldn't read file {}: {}", pd, e));
        let mut contents: Vec<FromFile<item::RawConfig>> = parse_and_merge_vec(&file)
            .unwrap_or_else(|e| fatal!("I don't like your YAML in {}: {}", pd, e))
            .into_iter()
            .map(|i| FromFile::new(i, pd.to_string()))
            .collect();
        info!("I like all {} items in {}!", contents.len(), pd);
        items.append(&mut contents);
    }

    items
}

pub(super) fn read_plants() -> Vec<FromFile<plant::RawConfig>> {
    let mut plants = vec![];

    for path in yml_files("plants") {
        let pd = path.display();
        let plant_name = path.file_stem().unwrap().to_str().unwrap();

        let skills_p = path.with_file_name(&format!("{}_skills.yml", plant_name));
        let skills_pd = skills_p.display();
        debug!("found {}, looking for {}", pd, skills_pd);
        let skills: Vec<plant::RawSkill> = match fs::read_to_string(&skills_p) {
            Ok(s) => {
                info!("reading plant config folder at {}", pd);
                match parse_and_merge_vec(&s) {
                    Err(e) => fatal!("I don't like your Skill YAML in {}: {}", skills_pd, e),
                    Ok(skills) => {
                        info!(
                            "I'm happy with all {} skills in {}!",
                            skills.len(),
                            skills_pd
                        );
                        skills
                    }
                }
            }
            Err(e) => {
                debug!(
                    "couldn't read skills, {} must not be plant folder: {}",
                    pd, e
                );
                continue;
            }
        };

        let file = fs::read_to_string(&path)
            .unwrap_or_else(|e| fatal!("\nCouldn't read file {}: {}", pd, e));
        let mut plant: plant::RawConfig = parse_and_merge(&file)
            .unwrap_or_else(|e| fatal!("I don't like your Plant YAML in {}: {}", pd, e));

        if plant.skills.len() > 0 {
            fatal!("plant skills must be defined in external file");
        }
        plant.skills = FromFile::new(skills, skills_pd.to_string());
        plants.push(FromFile::new(plant, pd.to_string()))
    }

    plants
}

fn yml_files(folder: &str) -> impl Iterator<Item = std::path::PathBuf> {
    let path = format!("{}/{}/", &*CONFIG_PATH, folder);
    info!("\nreading {}", path);
    walkdir::WalkDir::new(path)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| Some(e.ok()?.path().to_owned()))
        .filter(|p| {
            p.extension()
                .map(|e| e == "yml" || e == "yaml")
                .unwrap_or(false)
        })
}

fn parse_and_merge_vec<D: DeserializeOwned + fmt::Debug>(file: &str) -> Result<Vec<D>, String> {
    let values: Vec<Value> = serde_yaml::from_str(&file).map_err(|e| e.to_string())?;
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        let merged =
            yaml_merge_keys::merge_keys_serde(value).map_err(|e| format!("merge keys {}", e))?;

        output.push(parse_merged(merged)?)
    }
    Ok(output)
}

fn parse_and_merge<D: DeserializeOwned + fmt::Debug>(file: &str) -> Result<D, String> {
    let value = serde_yaml::from_str(&file).map_err(|e| e.to_string())?;
    let merged =
        yaml_merge_keys::merge_keys_serde(value).map_err(|e| format!("merge keys {}", e))?;
    parse_merged(merged)
}

/// Because anchors aren't officially part of the YAML spec, they're an extension,
/// and the Rust extension is external to the YAML parsing so there's no way for it to get the line numbers,
/// so this craziness is to resurrect line numbers in the errors to make debugging configs bearable
fn parse_merged<D: DeserializeOwned + fmt::Debug>(merged: Value) -> Result<D, String> {
    serde_yaml::from_value(merged.clone()).map_err(|_| {
        let ser = serde_yaml::to_string(&merged).unwrap();
        let err = serde_yaml::from_str::<D>(&ser).unwrap_err();
        let err_line = match err.location() {
            None => 0,
            Some(l) => l.line(),
        };
        let lines = ser
            .lines()
            .enumerate()
            .map(|(i, l)| {
                let mut line = l.to_string();
                let len_before = line.len();
                line.truncate(90);
                if len_before > 90 {
                    line.push_str("...")
                }
                format!("{:>3} | {}", i, line)
            })
            .skip(err_line.saturating_sub(10))
            .take(20)
            .collect::<Vec<_>>()
            .join("\n");
        let name = merged
            .get("name")
            .or(merged.get("title"))
            .and_then(|s| s.as_str());
        format!("\nname: {:?}\n{}\nerr: {}\n", name, lines, err)
    })
}
