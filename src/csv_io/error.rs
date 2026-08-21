//! Errors at the CSV processing boundary.

use crate::EngineError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
/// Failure while reading transactions or writing account results.
pub enum ProcessError {
    /// The input header record could not be read.
    #[error("could not read CSV headers: {0}")]
    ReadHeaders(#[source] csv::Error),
    /// The input did not contain exactly the required headers.
    #[error(
        "expected CSV headers type,client,tx,amount; found {}",
        .found.join(",")
    )]
    InvalidHeaders {
        /// Headers found in the input record.
        found: Vec<String>,
    },
    /// A CSV row could not be deserialized.
    #[error("could not read CSV row {row}: {source}")]
    ReadRow {
        /// One based row number, including the header row.
        row: usize,
        /// Deserialization error from the CSV reader.
        #[source]
        source: csv::Error,
    },
    /// A deserialized row violated the transaction shape.
    #[error("invalid CSV row {row}: {message}")]
    InvalidRow {
        /// One based row number, including the header row.
        row: usize,
        /// Human readable reason that the row is invalid.
        message: String,
    },
    /// A typed transaction could not be applied.
    #[error("could not apply CSV row {row}: {source}")]
    Apply {
        /// One based row number, including the header row.
        row: usize,
        /// Error returned by the ledger engine.
        #[source]
        source: EngineError,
    },
    /// Account snapshots could not be finalized.
    #[error("could not finalize accounts: {0}")]
    Finalize(#[source] EngineError),
    /// Account CSV could not be serialized or written.
    #[error("could not write account CSV: {0}")]
    Write(#[source] csv::Error),
    /// The final CSV buffer or underlying writer could not be flushed.
    #[error("could not flush account CSV: {0}")]
    Flush(#[source] io::Error),
}
