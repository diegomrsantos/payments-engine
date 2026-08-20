//! Exact account processing for chronological payment transactions.
//!
//! Use [`Engine`] when transactions already have typed values. Use
//! [`process_csv`] at an I/O boundary that receives and emits CSV data.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod account;
mod csv_io;
mod engine;
mod error;
mod model;
mod money;

pub use csv_io::{ProcessError, process_csv};
pub use engine::Engine;
pub use error::EngineError;
pub use model::{
    AccountSnapshot, ApplyOutcome, ClientId, IgnoreReason, Transaction, TransactionId,
};
