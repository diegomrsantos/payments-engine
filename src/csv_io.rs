//! Streaming CSV input and deterministic account output.

mod error;
mod input;
mod output;

pub use error::ProcessError;

use std::io::{Read, Write};

/// Processes transaction CSV data and writes the final account CSV data.
///
/// Input rows are read and applied incrementally in their original order.
/// Account output is delayed until the complete input has been validated, so
/// an input failure cannot produce a plausible partial result.
///
/// # Errors
///
/// Returns [`ProcessError`] for invalid headers or rows, ledger errors, and I/O
/// failures.
pub fn process_csv<R, W>(reader: R, writer: W) -> Result<(), ProcessError>
where
    R: Read,
    W: Write,
{
    let engine = input::read_engine(reader)?;
    let accounts = engine.accounts().map_err(ProcessError::Finalize)?;
    // Deposit metadata is no longer needed after the final snapshots exist.
    drop(engine);
    output::write_accounts(writer, accounts)
}
