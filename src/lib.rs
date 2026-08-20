//! Exact account processing for chronological payment transactions.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod account;
mod engine;
mod error;
mod model;
mod money;

pub use engine::Engine;
pub use error::EngineError;
pub use model::{
    AccountSnapshot, ApplyOutcome, ClientId, IgnoreReason, Transaction, TransactionId,
};
