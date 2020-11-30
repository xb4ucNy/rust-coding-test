use csv::{ReaderBuilder, Trim, Writer};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fs::File;
use std::{env, io};

pub mod client;
pub mod exchange;
pub mod transaction;

use crate::client::{Client, ClientId};
use crate::exchange::Exchange;
use crate::transaction::{Transaction, TransactionId};

/// This is a Data Transfer Object only used for CSV deserialization purposes.
#[derive(Deserialize)]
pub struct TransactionDTO {
    // "type" is a keyword, use "kind" instead
    #[serde(rename = "type")]
    pub kind: String,
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Option<f32>,
}

impl TryInto<Transaction> for TransactionDTO {
    type Error = String;
    fn try_into(self) -> Result<Transaction, String> {
        // The serde+csv combination can't deserialize into filled enums(?). Do
        // it manually instead.

        match self.kind.as_str() {
            "deposit" => {
                let amount = self.amount.ok_or(String::from("missing 'amount' field"))?;
                Ok(Transaction::Deposit(self.client, self.tx, amount))
            }
            "withdrawal" => {
                let amount = self.amount.ok_or(String::from("missing 'amount' field"))?;
                Ok(Transaction::Withdrawal(self.client, self.tx, amount))
            }
            "dispute" => Ok(Transaction::Dispute(self.client, self.tx)),
            "resolve" => Ok(Transaction::Resolve(self.client, self.tx)),
            "chargeback" => Ok(Transaction::Chargeback(self.client, self.tx)),
            _ => Err(String::from("unknown transaction type")),
        }
    }
}

/// This is a Data Transfer Object only used for CSV serialization purposes.
#[derive(Serialize)]
struct ClientDTO {
    client: ClientId,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

impl ClientDTO {
    fn new(id: &ClientId, client: &Client) -> ClientDTO {
        ClientDTO {
            client: *id,
            available: client.funds_available,
            held: client.funds_held,
            total: client.funds_total(),
            locked: client.locked,
        }
    }
}

fn main() {
    let input_filename = env::args().nth(1).expect("no filename provided");
    let input_file = File::open(input_filename).expect("could not open file");
    let mut input = ReaderBuilder::new()
        // remove whitespace when reading headers and values, otherwise they may
        // be read incorrectly
        .trim(Trim::All)
        // allow rows to be different sizes (dispute, resolve, chargeback don't
        // include an "amount" field)
        .flexible(true)
        .from_reader(input_file);

    let mut exchange = Exchange::new();

    for row in input.deserialize::<TransactionDTO>() {
        let transaction = row
            .expect("failed to read row")
            .try_into()
            .expect("failed to read row");

        match exchange.process(transaction) {
            Err(_) => {
                // just swallow logs for now, in the long term they should be
                // logged somewhere.
            }
            _ => {}
        }
    }

    let mut output = Writer::from_writer(io::stdout());

    for (id, client) in exchange.clients() {
        let dto = ClientDTO::new(id, client);
        output.serialize(dto).expect("failed to write row");
    }
}
