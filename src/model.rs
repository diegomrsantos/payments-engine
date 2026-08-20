//! Public transaction and account value types.

use rust_decimal::Decimal;

/// Identifier for a client account.
pub type ClientId = u16;
/// Identifier for a transaction.
pub type TransactionId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A typed transaction that can be applied to an [`Engine`](crate::Engine).
pub enum Transaction {
    /// Adds funds to a client account.
    Deposit {
        /// Account that receives the funds.
        client: ClientId,
        /// Globally unique transaction identifier from the input contract.
        tx: TransactionId,
        /// Positive amount with at most four decimal places.
        amount: Decimal,
    },
    /// Removes available funds when the account has a sufficient balance.
    Withdrawal {
        /// Account that supplies the funds.
        client: ClientId,
        /// Identifier from the input transaction.
        tx: TransactionId,
        /// Positive amount with at most four decimal places.
        amount: Decimal,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Result of a valid transaction request.
pub enum ApplyOutcome {
    /// The transaction changed ledger state.
    Applied,
    /// The requested balance change did not apply.
    Ignored(IgnoreReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Reason that a requested balance change did not apply.
pub enum IgnoreReason {
    /// The withdrawal exceeded the available balance.
    ///
    /// If the client has no account yet, the withdrawal establishes an empty one.
    InsufficientFunds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Read only view of a client account after processing.
pub struct AccountSnapshot {
    /// Client account identifier.
    pub client: ClientId,
    /// Funds available for withdrawal.
    pub available: Decimal,
    /// Funds held by active disputes.
    pub held: Decimal,
    /// Sum of available and held funds, computed rather than stored.
    pub total: Decimal,
    /// Whether the account is locked.
    pub locked: bool,
}
