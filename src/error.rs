//! Errors that protect ledger arithmetic and invariants.

use crate::money::SCALE;
use crate::{ClientId, TransactionId};
use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
/// Failure to apply or inspect exact ledger state.
pub enum EngineError {
    /// A monetary amount was zero or negative.
    #[error("amount must be positive, got {amount}")]
    InvalidAmount {
        /// Amount that failed validation.
        amount: Decimal,
    },
    /// A monetary amount had more than four decimal places.
    #[error(
        "amount {amount} has {scale} decimal places; at most {max_scale} are allowed",
        max_scale = SCALE
    )]
    ExcessPrecision {
        /// Amount that failed validation.
        amount: Decimal,
        /// Number of decimal places in the amount.
        scale: u32,
    },
    /// Exact decimal arithmetic exceeded the supported range.
    #[error("balance arithmetic overflow for client {client}")]
    ArithmeticOverflow {
        /// Account whose balance could not be represented.
        client: ClientId,
    },
    /// Internal account state did not satisfy a required invariant.
    #[error("balance invariant violated for client {client}")]
    InvariantViolation {
        /// Account that did not satisfy the invariant.
        client: ClientId,
    },
    /// A deposit reused an identifier that already belongs to a deposit.
    #[error("duplicate deposit transaction ID {tx}")]
    DuplicateTransaction {
        /// Reused transaction identifier.
        tx: TransactionId,
    },
}
