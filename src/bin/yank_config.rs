#![recursion_limit = "256"]
use hcor::yank_config::{yank_config, YankError};

#[tokio::main]
async fn main() -> Result<(), YankError> {
    yank_config().await
}
