#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sale {
    pub price: u64,
    pub market_name: String,
}
