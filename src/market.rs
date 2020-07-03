#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
/// Represents something put up for sale by a hackagotchi player.
pub struct Sale {
    /// How much currency must be paid to acquire this item.
    pub price: u64,
    /// The name this item assumes for marketing purposes.
    pub market_name: String,
}
