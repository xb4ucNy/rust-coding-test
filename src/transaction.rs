use crate::client::ClientId;

/// Transactions are identified by a unique 32-bit number.
pub type TransactionId = u32;

/// Represents the types of transactions (and their associated data) that can be
/// used with an Exchange.
pub enum Transaction {
    Deposit(ClientId, TransactionId, f32),
    Withdrawal(ClientId, TransactionId, f32),
    Dispute(ClientId, TransactionId),
    Resolve(ClientId, TransactionId),
    Chargeback(ClientId, TransactionId),
}
