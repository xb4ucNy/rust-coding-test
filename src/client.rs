pub type ClientId = u16;

/// Represents a client's account.
#[derive(Debug, PartialEq)]
pub struct Client {
    /// The total funds that are available for trading, staking, withdrawal,
    /// etc.
    pub funds_available: f32,

    /// The total funds that are held for dispute.
    pub funds_held: f32,

    /// Whether the account is locked. An account is locked if a charge back
    /// occurs.
    pub locked: bool,
}

impl Client {
    /// Creates an empty client with no funds and not locked.
    pub fn new() -> Client {
        Client {
            funds_available: 0.0,
            funds_held: 0.0,
            locked: false,
        }
    }

    pub fn funds_total(&self) -> f32 {
        self.funds_available + self.funds_held
    }
}

impl Default for Client {
    fn default() -> Client {
        Client::new()
    }
}
