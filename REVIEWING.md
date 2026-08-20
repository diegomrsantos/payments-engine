# Review Guide

This guide explains how to evaluate the implementation. Read the
[README](README.md) first for the external contract and settled behavior. Use
the [architecture guide](ARCHITECTURE.md) for component ownership, data flow,
and the deposit lifecycle.

## Review principles

- Establish intent and invariants before judging implementation details.
- Inspect the areas with the greatest correctness risk first.
- Read tests beside the behavior they specify, then inspect surrounding source
  rather than only the changed lines.
- Treat automated checks as evidence, not as a substitute for reasoning.
- Separate contract, correctness, and safety findings from optional clarity or
  style suggestions.
- Review the integrated behavior first. Use focused commits to understand intent
  and scope, then return to the cumulative tree for a final integrated check.

## Fifteen minute orientation

This path establishes the mental model. It is not a complete review.

| Time | Read | Goal |
| --- | --- | --- |
| 0:00–2:00 | Quick start and CSV contract in the [README](README.md), then [src/lib.rs](src/lib.rs) | Establish the supported interface and external behavior. |
| 2:00–4:00 | Diagrams and component contracts in the [architecture guide](ARCHITECTURE.md) | Identify ownership and follow one record through the system. |
| 4:00–7:00 | [src/engine.rs](src/engine.rs), [transaction stories](src/engine/tests/transactions.rs), and [dispute stories](src/engine/tests/disputes.rs) | Review chronological coordination and the deposit state machine. |
| 7:00–9:00 | [src/account.rs](src/account.rs), [src/money.rs](src/money.rs), and their focused tests | Locate the balance mutation boundary and exact arithmetic guarantees. |
| 9:00–12:00 | [src/csv_io.rs](src/csv_io.rs), [input](src/csv_io/input.rs), [output](src/csv_io/output.rs), and [CSV stories](tests/csv_processing.rs) | Trace incremental input to deterministic final output. |
| 12:00–15:00 | [src/main.rs](src/main.rs), [CLI stories](tests/cli.rs), and [I/O failure stories](tests/io_failures.rs) | Confirm stdout, stderr, exit status, and partial writer behavior. |

## Complete review order

### 1. Domain behavior

Start with [src/engine.rs](src/engine.rs) and its transaction and dispute tests.
Confirm that only successful deposits are disputable, controls validate client
ownership, resolve permits a later dispute, chargeback locks the account, and
later otherwise valid operations for that client are ignored. Check that other
clients continue independently.

### 2. Monetary invariants

Review [src/money.rs](src/money.rs) with [money tests](src/money/tests.rs), then
[src/account.rs](src/account.rs) with [account tests](src/account/tests.rs).
Confirm exact units of `0.0001`, checked arithmetic, atomic balance replacement,
nonnegative held funds, computed totals, and intentional negative available
funds after spent deposits are disputed. Use
[tests/engine_properties.rs](tests/engine_properties.rs) for invariants across
components and numeric examples.

### 3. Input, output, and failures

Review the CSV boundary with [input validation](tests/input_validation.rs),
[CSV processing](tests/csv_processing.rs), and
[I/O failures](tests/io_failures.rs). Confirm incremental parsing, row context,
exact decimal spelling rules, deterministic output, delayed output for input
failures, and the unavoidable limit on partial output after a writer accepts
bytes. Review [src/main.rs](src/main.rs) with [CLI stories](tests/cli.rs) for
argument count, diagnostics, and exit status.

### 4. Streaming and concurrency

Inspect [src/csv_io/input.rs](src/csv_io/input.rs),
[src/engine.rs](src/engine.rs), and [streaming tests](tests/streaming.rs).
Confirm that input rows are not collected, memory remains proportional to
clients, applied deposits, and the largest buffered record, and row order stays
sequential within one engine. The worker test in
[tests/engine_properties.rs](tests/engine_properties.rs) demonstrates that
independent engines can run on separate threads without shared state.

### 5. Dependencies and automation

Inspect [Cargo.toml](Cargo.toml), [deny.toml](deny.toml), and
[the CI workflow](.github/workflows/ci.yml). Confirm the minimum Rust version,
locked dependency use, immutable action revisions, disabled checkout
credentials, restricted dependency sources, advisory checks, and the explicit
ban on unwanted `rust_decimal` archive features.

### 6. Maintainability and documentation

Check that each module owns one concept, domain code has no CSV or command line
knowledge, tests use semantic scenario names, and helpers remain shallow. Read
all source comments and Rustdoc beside the code they describe. Confirm that the
README states external behavior while the architecture guide explains internal
collaboration.

## Reviewing the commits

After reviewing the integrated behavior above, use the focused commits to
understand why each part changed:

```sh
git log --reverse --oneline --stat
git show --stat <commit>
git show <commit>
```

For each commit, check that its description explains intent, its tests travel
with its behavior, and no unrelated cleanup obscures the change. Then choose
the scaffold or other intended base revision from the log and review the
complete result so interactions across commits are not missed:

```sh
git diff --stat <base>...HEAD
git diff --check <base>...HEAD
git diff <base>...HEAD
```

For local work that is not committed, inspect scope before details:

```sh
git status --short
git diff --stat
git diff --check
git diff
```

## Reporting findings

Use the impact, not personal preference, to classify a finding:

- **Blocking:** Violates the contract, corrupts ledger state, risks data loss or
  security, or leaves the project unable to build or run as documented.
- **Important:** Creates a realistic correctness, testing, maintainability, or
  documentation risk that should be resolved before approval.
- **Suggestion:** Improves clarity, style, or a future extension but is not
  required for approval.

Every finding should identify a precise location, observable impact, supporting
evidence or reproduction, and the smallest useful direction for a fix. The
final review summary should state what was reviewed, which commands ran, all
remaining findings by level, assumptions or residual risks, and an explicit
approval or request for changes.

## Verification

Check the declared minimum Rust version:

```sh
cargo +1.85.0 check --all-targets --locked
```

Run the complete quality set:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --lib --locked
cargo build --release --locked
```

When `cargo-deny` is installed, run the dependency policy:

```sh
cargo deny --locked check advisories bans sources
```

The generated library reference is available at
`target/doc/payments_engine/index.html`. The CI workflow runs the same quality
and dependency policy checks on pushes and pull requests.
