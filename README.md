# Payments Engine

A small Rust ledger that processes chronological payment transactions from CSV
and writes one final balance row per client. The design favors explicit state
transitions, exact arithmetic, and code that can be reviewed without additional
explanation.

Use the [architecture guide](ARCHITECTURE.md) for the system mental model and
component collaboration.

## Quick start

The project requires Rust 1.85 or newer.

```sh
cargo build --locked
cargo run -- transactions.csv > accounts.csv
```

The command accepts exactly one input path. Account CSV data is the only output
written to stdout. Usage errors and processing failures are written to stderr
and return a nonzero status.

## CSV contract

Input must contain exactly the headers `type,client,tx,amount`. Header order may
vary, and surrounding whitespace is ignored. Rows are processed in their
written order; transaction identifiers do not define chronology and need not
increase.

Output always uses this order:
`client,available,held,total,locked`. Client rows are sorted in ascending order,
and balances are written with four decimal places.

Deposit and withdrawal amounts must be positive exact decimal values with no
more than four written decimal places. Dispute, resolve, and chargeback rows
must leave `amount` blank. Client identifiers must fit `u16`, and transaction
identifiers must fit `u32`. Plain decimal notation is required; scientific
notation is rejected. Written precision is checked before trailing fractional
zeroes are removed for exact parsing, so equivalent large values remain valid.

## Transaction behavior

- A deposit adds its amount to available funds.
- A withdrawal applies only when sufficient available funds exist.
- Only successful deposits can be disputed. Withdrawals are not disputable.
- A control operation applies only when its client matches the owner of the
  referenced deposit.
- A dispute moves the complete deposit amount from available to held funds.
- A resolve returns that amount to available funds. The deposit may be disputed
  again because resolve reverses the hold rather than the original deposit.
- A chargeback removes the referenced amount from held funds and locks the
  account. Locking is terminal, so later otherwise valid operations for that
  client are ignored while other clients continue.

A dispute may make available funds negative when some of the deposited funds
have already been withdrawn. Held funds cannot become negative. A chargeback
removes only the amount for its referenced deposit, so other held deposits can
remain visible on the newly locked account.

## Validation and failure policy

Unknown references, client mismatches, insufficient withdrawals, invalid
dispute transitions, and otherwise valid operations on a locked account are
valid requests whose requested balance or dispute transition cannot apply. They
are ignored and processing continues. Validation and reference checks can occur
before the lock check, so the lock rule applies to operations that are otherwise
valid.

A first withdrawal establishes its client before the available balance check.
If funds are insufficient, it can therefore leave a new zero balance account.
Control operations only reference existing deposits, so an unknown control
reference does not establish an account.

Malformed CSV, unexpected headers, unknown transaction types, invalid amounts,
arithmetic that cannot remain exact, and a deposit identifier that would
overwrite retained dispute metadata are errors. The complete input is validated
and every account row is formatted before account output begins. These failures
cannot leave a plausible partial result on stdout. A writer or flush failure can
still occur after output begins because a generic output stream cannot make the
complete write atomic.

The input contract requires the primary transaction identifiers on deposit and
withdrawal rows to be globally unique. The `tx` field on a dispute, resolve, or
chargeback row instead references the existing deposit that it controls; it is
not a new primary transaction identifier. The engine trusts primary identifier
uniqueness rather than retaining a separate set of every identifier. Metadata
for applied deposits is already retained for later control operations, so a
deposit that reuses an identifier in that set is rejected. Withdrawal
identifiers and identifiers from ignored deposits are not retained.

## Exactness and operating model

Amounts are parsed without binary floating point conversion. Ledger arithmetic
uses exact units of `0.0001`, checked operations, and a computed total. Account
CSV formatting always emits four decimal places without rounding.

Input is consumed one row at a time rather than collected in memory. Rows within
one stream remain sequential because their written order defines ledger order.
Each `Engine` owns its state, so independent streams can run concurrently in a
server without sharing ledger state. Combining several streams into one ledger
would require a separate ordering contract.

Each engine represents one implicit asset. The CSV format and public model have
no asset or currency field, so a caller must route different assets to separate
engine instances.

## Production considerations

This crate deliberately implements the compact ledger described above. A
service built around it would normally add:

- durable storage, atomic ledger commits, an audit history, and recovery;
- an explicit ordering and identifier contract when several connections feed
  one ledger;
- idempotency and retry handling at the service boundary;
- authentication, authorization, framing, size limits, timeouts, backpressure,
  and rate limits;
- formal models or proof harnesses for critical invariants and distributed
  ordering protocols when higher assurance justifies their maintenance; and
- operational metrics and explicit asset identifiers when more than one asset
  is supported.

These concerns belong around the ledger rather than inside its CSV processing
interface.

## Test strategy

Small unit stories explain money, account, and engine rules. Integration tests
cover CSV processing, validation, command behavior, incremental reads, and a
large generated stream. Separate worker tests show that engine instances can
move across threads without shared state.

These tests provide focused evidence for the specified behavior, but no finite
suite proves that defects are absent. A local mutation audit helped identify
missing boundary stories during development. For a longer lived system,
property based testing and recurring mutation analysis could provide additional
confidence without changing the ledger design.

## Development disclosure

Generative AI was used during planning, implementation, and review. A private
record of the material AI contributions is supplied separately with the
submission.
