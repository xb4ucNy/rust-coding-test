use crate::client::{Client, ClientId};
use crate::transaction::{Transaction, TransactionId as TxId};
use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Eq, PartialEq)]
pub enum ExchangeError {
    /// A transaction already exists with that ID.
    TransactionAlreadyExists,

    /// The original transaction has already been disputed and cannot be
    /// disputed again.
    TransactionAlreadyDisputed,

    /// The original transaction has not been disputed so Resolve or Chargeback
    /// transactions are invalid.
    TransactionNotDisputed,

    /// No transaction with that ID exists.
    TransactionNotFound,

    /// The client does not have enough funds to fulfill the transaction.
    InsufficientFunds,
}

use ExchangeError::*;

/// Used by the exchange to keep track of transaction history
enum TransactionState {
    /// The transaction has been processed.
    Completed(f32),

    /// The transaction has been disputed. The funds are held until the dispute
    /// is resolved.
    Disputed(f32),

    /// The transaction had a dispute that has been resolved, either by a
    /// Resolve or Chargeback transaction.
    Resolved,
}

use TransactionState::*;

/// The exchange handles all transactions.
///
/// It keeps track of clients and transaction history. It handles deposits,
/// withdrawals, and the dispute resolution process. All actions are done via
/// transactions.
pub struct Exchange {
    transactions: HashMap<TxId, TransactionState>,
    clients: HashMap<ClientId, Client>,
}

impl Exchange {
    /// Creates an empty exchange.
    pub fn new() -> Exchange {
        Exchange {
            transactions: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    pub fn process(&mut self, transaction: Transaction) -> Result<(), ExchangeError> {
        use Transaction::*;

        match transaction {
            Deposit(client, tx, amount) => self.deposit(tx, client, amount),
            Withdrawal(client, tx, amount) => self.withdraw(tx, client, amount),
            Dispute(client, tx) => self.dispute(tx, client),
            Resolve(client, tx) => self.resolve(tx, client),
            Chargeback(client, tx) => self.chargeback(tx, client),
        }
    }

    pub fn clients(&self) -> impl Iterator<Item = (&ClientId, &Client)> {
        self.clients.iter()
    }

    fn deposit(&mut self, tx: TxId, client: ClientId, amount: f32) -> Result<(), ExchangeError> {
        let client = self.clients.entry(client).or_default();

        match self.transactions.entry(tx) {
            Entry::Occupied(_) => return Err(TransactionAlreadyExists),
            Entry::Vacant(entry) => entry.insert(Completed(amount)),
        };

        client.funds_available += amount;

        Ok(())
    }

    fn withdraw(&mut self, tx: TxId, client: ClientId, amount: f32) -> Result<(), ExchangeError> {
        let client = self.clients.entry(client).or_default();

        if client.funds_available < amount {
            return Err(InsufficientFunds);
        }

        match self.transactions.entry(tx) {
            Entry::Occupied(_) => return Err(TransactionAlreadyExists),
            Entry::Vacant(entry) => entry.insert(Completed(-amount)),
        };

        client.funds_available -= amount;

        Ok(())
    }

    fn dispute(&mut self, tx: TxId, client: ClientId) -> Result<(), ExchangeError> {
        let state = self.transactions.get_mut(&tx).ok_or(TransactionNotFound)?;
        let client = self.clients.entry(client).or_default();

        let amount = match state {
            Completed(amount) => *amount,
            _ => return Err(TransactionAlreadyDisputed),
        };

        *state = Disputed(amount);
        client.funds_available -= amount;
        client.funds_held += amount;

        Ok(())
    }

    fn resolve(&mut self, tx: TxId, client: ClientId) -> Result<(), ExchangeError> {
        let state = self.transactions.get_mut(&tx).ok_or(TransactionNotFound)?;
        let client = self.clients.entry(client).or_default();

        let amount = match state {
            Disputed(amount) => *amount,
            _ => return Err(TransactionNotDisputed),
        };

        *state = Resolved;
        client.funds_available += amount;
        client.funds_held -= amount;

        Ok(())
    }

    fn chargeback(&mut self, tx: TxId, client: ClientId) -> Result<(), ExchangeError> {
        let state = self.transactions.get_mut(&tx).ok_or(TransactionNotFound)?;
        let client = self.clients.entry(client).or_default();

        let amount = match state {
            Disputed(amount) => *amount,
            _ => return Err(TransactionNotDisputed),
        };

        *state = Resolved;
        client.funds_held -= amount;
        client.locked = true;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_succeeds_and_adds_funds_with_unique_tx_id() {
        let mut exchange = Exchange::new();

        assert!(exchange.deposit(5, 1, 1.0).is_ok());

        let client = exchange.clients.get(&1).unwrap();
        assert_eq!(client.funds_held, 0.0);
        assert_eq!(client.funds_available, 1.0);
    }

    #[test]
    fn deposit_fails_with_non_unique_tx_id() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        assert_eq!(exchange.deposit(5, 1, 2.0), Err(TransactionAlreadyExists));

        exchange.withdraw(6, 1, 1.0).unwrap();
        assert_eq!(exchange.deposit(6, 1, 2.0), Err(TransactionAlreadyExists));
    }

    #[test]
    fn withdraw_succeeds_and_pulls_funds_with_unique_tx_id() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        assert!(exchange.withdraw(6, 1, 1.0).is_ok());

        let client = exchange.clients.get(&1).unwrap();
        assert_eq!(client.funds_held, 0.0);
        assert_eq!(client.funds_available, 0.0);
    }

    #[test]
    fn withdraw_fails_with_non_unique_id() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 4.0).unwrap();
        assert_eq!(exchange.withdraw(5, 1, 1.0), Err(TransactionAlreadyExists));

        exchange.withdraw(6, 1, 2.0).unwrap();
        assert_eq!(exchange.withdraw(6, 1, 1.0), Err(TransactionAlreadyExists));
    }

    #[test]
    fn withdraw_fails_if_client_has_insufficient_funds() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        assert_eq!(exchange.withdraw(6, 1, 2.0), Err(InsufficientFunds));

        let client = exchange.clients.get(&1).unwrap();
        assert_eq!(client.funds_held, 0.0);
        assert_eq!(client.funds_available, 1.0);
    }

    #[test]
    fn dispute_succeeds_and_holds_funds_on_existing_transaction() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        assert!(exchange.dispute(5, 1).is_ok());

        let client = exchange.clients.get(&1).unwrap();
        assert_eq!(client.funds_held, 1.0);
        assert_eq!(client.funds_available, 0.0);
    }

    #[test]
    fn dispute_fails_if_transaction_doesnt_exist() {
        let mut exchange = Exchange::new();

        assert_eq!(exchange.dispute(5, 1), Err(TransactionNotFound));
    }

    #[test]
    fn dispute_fails_if_transaction_is_already_disputed() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        exchange.dispute(5, 1).unwrap();
        assert_eq!(exchange.dispute(5, 1), Err(TransactionAlreadyDisputed));
    }

    #[test]
    fn dispute_fails_if_transaction_is_already_resolved() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        exchange.dispute(5, 1).unwrap();
        exchange.resolve(5, 1).unwrap();
        assert_eq!(exchange.dispute(5, 1), Err(TransactionAlreadyDisputed));

        exchange.deposit(6, 1, 1.0).unwrap();
        exchange.dispute(6, 1).unwrap();
        exchange.chargeback(6, 1).unwrap();
        assert_eq!(exchange.dispute(6, 1), Err(TransactionAlreadyDisputed));
    }

    #[test]
    fn resolve_succeeds_and_releases_held_funds_on_disputed_transaction() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        exchange.dispute(5, 1).unwrap();
        assert!(exchange.resolve(5, 1).is_ok());

        let client = exchange.clients.get(&1).unwrap();
        assert_eq!(client.funds_held, 0.0);
        assert_eq!(client.funds_available, 1.0);
    }

    #[test]
    fn resolve_fails_if_transaction_doesnt_exists() {
        let mut exchange = Exchange::new();

        assert_eq!(exchange.resolve(5, 1), Err(TransactionNotFound));
    }

    #[test]
    fn resolve_fails_if_transaction_is_not_disputed() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        assert_eq!(exchange.resolve(5, 1), Err(TransactionNotDisputed));
    }

    #[test]
    fn resolve_fails_if_transaction_already_resolved() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        exchange.dispute(5, 1).unwrap();
        exchange.resolve(5, 1).unwrap();
        assert_eq!(exchange.resolve(5, 1), Err(TransactionNotDisputed));

        exchange.deposit(6, 1, 1.0).unwrap();
        exchange.dispute(6, 1).unwrap();
        exchange.chargeback(6, 1).unwrap();
        assert_eq!(exchange.resolve(6, 1), Err(TransactionNotDisputed));
    }

    #[test]
    fn chargeback_succeeds_and_removes_held_funds_and_locks_client_on_disputed_transaction() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        exchange.dispute(5, 1).unwrap();
        assert!(exchange.chargeback(5, 1).is_ok());

        let client = exchange.clients.get(&1).unwrap();
        assert_eq!(client.funds_held, 0.0);
        assert_eq!(client.funds_available, 0.0);
        assert_eq!(client.locked, true);
    }

    #[test]
    fn chargeback_fails_if_transaction_doesnt_exists() {
        let mut exchange = Exchange::new();

        assert_eq!(exchange.chargeback(5, 1), Err(TransactionNotFound));
    }

    #[test]
    fn chargeback_fails_if_transaction_is_not_disputed() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        assert_eq!(exchange.chargeback(5, 1), Err(TransactionNotDisputed));
    }

    #[test]
    fn chargeback_fails_if_transaction_already_resolved() {
        let mut exchange = Exchange::new();

        exchange.deposit(5, 1, 1.0).unwrap();
        exchange.dispute(5, 1).unwrap();
        exchange.resolve(5, 1).unwrap();
        assert_eq!(exchange.chargeback(5, 1), Err(TransactionNotDisputed));

        exchange.deposit(6, 1, 1.0).unwrap();
        exchange.dispute(6, 1).unwrap();
        exchange.chargeback(6, 1).unwrap();
        assert_eq!(exchange.chargeback(6, 1), Err(TransactionNotDisputed));
    }

    #[test]
    fn clients_returns_all_clients() {
        let mut exchange = Exchange::new();

        exchange.deposit(0, 1, 1.0).unwrap();
        exchange.deposit(1, 2, 2.0).unwrap();
        exchange.deposit(2, 5, 4.0).unwrap();
        exchange.withdraw(3, 2, 1.0).unwrap();

        let clients = exchange.clients().collect::<Vec<_>>();
        assert_eq!(
            clients.iter().find(|(&k, _)| k == 1).map(|(_, v)| *v),
            Some(&Client {
                funds_available: 1.0,
                funds_held: 0.0,
                locked: false,
            })
        );
        assert_eq!(
            clients.iter().find(|(&k, _)| k == 2).map(|(_, v)| *v),
            Some(&Client {
                funds_available: 1.0,
                funds_held: 0.0,
                locked: false,
            })
        );
        assert_eq!(
            clients.iter().find(|(&k, _)| k == 5).map(|(_, v)| *v),
            Some(&Client {
                funds_available: 4.0,
                funds_held: 0.0,
                locked: false,
            })
        );
    }
}
