//! Errors that protect ledger arithmetic and invariants.

use crate::money::SCALE;
use crate::{ClientId, TransactionId};
use rust_decimal::Decimal;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Failure to apply or inspect exact ledger state.
pub enum EngineError {
    /// A monetary amount was zero or negative.
    InvalidAmount {
        /// Amount that failed validation.
        amount: Decimal,
    },
    /// A monetary amount had more than four decimal places.
    ExcessPrecision {
        /// Amount that failed validation.
        amount: Decimal,
        /// Number of decimal places in the amount.
        scale: u32,
    },
    /// Exact decimal arithmetic exceeded the supported range.
    ArithmeticOverflow {
        /// Account whose balance could not be represented.
        client: ClientId,
    },
    /// Internal account state did not satisfy a required invariant.
    InvariantViolation {
        /// Account that did not satisfy the invariant.
        client: ClientId,
    },
    /// A deposit reused an identifier that already belongs to a deposit.
    DuplicateTransaction {
        /// Reused transaction identifier.
        tx: TransactionId,
    },
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAmount { amount } => {
                write!(formatter, "amount must be positive, got {amount}")
            }
            Self::ExcessPrecision { amount, scale } => write!(
                formatter,
                "amount {amount} has {scale} decimal places; at most {SCALE} are allowed"
            ),
            Self::ArithmeticOverflow { client } => {
                write!(formatter, "balance arithmetic overflow for client {client}")
            }
            Self::InvariantViolation { client } => {
                write!(formatter, "balance invariant violated for client {client}")
            }
            Self::DuplicateTransaction { tx } => {
                write!(formatter, "duplicate deposit transaction ID {tx}")
            }
        }
    }
}

impl Error for EngineError {}
