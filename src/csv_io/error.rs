//! Errors at the CSV processing boundary.

use crate::EngineError;
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
/// Failure while reading transactions or writing account results.
pub enum ProcessError {
    /// The input header record could not be read.
    ReadHeaders(csv::Error),
    /// The input did not contain exactly the required headers.
    InvalidHeaders {
        /// Headers found in the input record.
        found: Vec<String>,
    },
    /// A CSV row could not be deserialized.
    ReadRow {
        /// One based row number, including the header row.
        row: usize,
        /// Deserialization error from the CSV reader.
        source: csv::Error,
    },
    /// A deserialized row violated the transaction shape.
    InvalidRow {
        /// One based row number, including the header row.
        row: usize,
        /// Human readable reason that the row is invalid.
        message: String,
    },
    /// A typed transaction could not be applied.
    Apply {
        /// One based row number, including the header row.
        row: usize,
        /// Error returned by the ledger engine.
        source: EngineError,
    },
    /// Account snapshots could not be finalized.
    Finalize(EngineError),
    /// Account CSV could not be serialized or written.
    Write(csv::Error),
    /// The final CSV buffer or underlying writer could not be flushed.
    Flush(io::Error),
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadHeaders(source) => write!(formatter, "could not read CSV headers: {source}"),
            Self::InvalidHeaders { found } => write!(
                formatter,
                "expected CSV headers type,client,tx,amount; found {}",
                found.join(",")
            ),
            Self::ReadRow { row, source } => {
                write!(formatter, "could not read CSV row {row}: {source}")
            }
            Self::InvalidRow { row, message } => {
                write!(formatter, "invalid CSV row {row}: {message}")
            }
            Self::Apply { row, source } => {
                write!(formatter, "could not apply CSV row {row}: {source}")
            }
            Self::Finalize(source) => write!(formatter, "could not finalize accounts: {source}"),
            Self::Write(source) => write!(formatter, "could not write account CSV: {source}"),
            Self::Flush(source) => write!(formatter, "could not flush account CSV: {source}"),
        }
    }
}

impl Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadHeaders(source) | Self::Write(source) => Some(source),
            Self::ReadRow { source, .. } => Some(source),
            Self::Apply { source, .. } | Self::Finalize(source) => Some(source),
            Self::Flush(source) => Some(source),
            Self::InvalidHeaders { .. } | Self::InvalidRow { .. } => None,
        }
    }
}
