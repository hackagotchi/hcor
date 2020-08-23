use std::fs;

fn main() {
    pretty_env_logger::init();

    let start = std::time::Instant::now();
    match hcor::config::yaml_and_verify() {
        Err(e) => println!("{}", e),
        Ok(config) => {
            println!("Love this config!");

            write_config_json(&config);
            write_config_bincode(&config).unwrap();
        }
    }
    let elapsed = start.elapsed();
    log::info!("Elapsed: {:?}", elapsed);
}

fn write_config_json(config: &hcor::config::Config) {
    let path = format!("{}/config.json", &*hcor::config::CONFIG_PATH);

    println!("Transpiling it to {}", path);
    match serde_json::to_string(config).map(|j| fs::write(&path, j)) {
        Ok(Err(e)) => println!("couldn't write JSON to {}: {}", path, e),
        Err(e) => println!("couldn't transpile JSON: {}", e),
        Ok(_) => println!("Alright, all done!"),
    }
}

fn write_config_bincode(config: &hcor::config::Config) -> Result<(), String> {
    let path = format!("{}/config.bincode", &*hcor::config::CONFIG_PATH);
    println!("Transpiling it to {}", path);

    let compressed = zstd::encode_all(
        bincode::serialize(config)
            .map_err(|e| format!("couldn't transpile bincode: {}", e))?
            .as_slice(),
        10
    )
    .map_err(|e| format!("couldn't compress bincode: {}", e))?;
    println!("compressed len: {}", compressed.len());

    fs::write(&path, &compressed)
        .map_err(|e| format!("couldn't write bincode to {}: {}", path, e))?;

    println!("Alright, all done!");
    Ok(())
}
