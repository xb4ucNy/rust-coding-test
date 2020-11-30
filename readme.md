# Rust Coding Test

Built using Rust 1.48.0

- `cargo run -- example.csv`
- `cargo doc --open`
- `cargo test`

## Design

I chose to design this program with `main.rs` facilitating the CSV input and
output but left the rest of the code agnostic to it. So it reads rows and
converts them to the `Transaction` enum (built do avoid issues with optional
`amount`s); and converts and writes rows from the `Client` struct so that client
storage is separate from the output format.

The `Exchange` class does all the non-csv processing. I have the `process`
function split out into helper functions to aid in organization and unit
testing. It creates and stores the clients as needed and since it has to store
transactions history for the dispute resolution process, I ensured that it only
stored what is necessary.

## Concerns

- I did not take advantage of the "four places past the decimal" precision. The
  code as written takes the easy path and just used `f32` for amounts. I would
  be much more confident in the financial transactions if I avoided floating
  point altogether. But I didn't take the time to do it and convert to integer
  milli-units to and from the CSV format.

- I left a gap in the dispute process, the `client` that is affected is always
  what is provided in the dispute/resolve/chargeback and it doesn't check that
  it matches the original transaction. This potential issue wasn't brought up in
  the doc but as I wrote it would be exploitable.

- The wording for the dispute process somewhat implies that its only valid for
  deposits, though I can image that a withdrawal could be disputed as well (they
  are both "primary" transactions). I followed the wording though but allowed
  withdrawals to be disputed, but it therefore calculates it by having negative
  funds held.

- It wasn't clear if a transaction should be able to be disputed a second time
  even if it was resolved. The way I implemented it disallowed that and returns
  a `TransactionAlreadyDisputed` error.

- If an account is "locked" I would imagine that it would have some
  functionality limited, but that wasn't mentioned in the doc. Also, the
  description for chargebacks used the word "frozen" instead of "locked".

- Persisting clients and transactions are my only concerns for potential
  performance issues, though I'm not sure how it can be avoided without a
  separate database system. I used `HashMap`s for the storage so it should be
  good enough for a start.

- For logging and debugging purposes, I would probably make the `ExchangeError`s
  provide more information. Right now it only returns `TransactionDoesntExist`
  for example but doesn't include what transaction it was looking for.

- I left `TransactionId` and `ClientId` just as type aliases for `u32` and `u16`
  but it might be better to use newtype structs so they can't be confused. Right
  now unit test writing could easily mess up the arguments.
