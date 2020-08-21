use std::fs;

fn main() {
    pretty_env_logger::init();

    let start = std::time::Instant::now();
    match hcor::config::yaml_and_verify() {
        Err(e) => println!("{}", e),
        Ok(config) => {
            let path = format!("{}/config.json", &*hcor::config::CONFIG_PATH);

            println!("Love this config! Transpiling it to {}", path);
            match serde_json::to_string(&config).map(|j| fs::write(&path, j)) {
                Ok(Err(e)) => println!("couldn't write JSON to {}: {}", path, e),
                Err(e) => println!("couldn't transpile JSON: {}", e),
                Ok(_) => println!("Alright, all done!"),
            }
        }
    }
    let elapsed = start.elapsed();
    log::info!("Elapsed: {:?}", elapsed);
}
