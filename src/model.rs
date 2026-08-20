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
        ///
        /// Withdrawals are not disputable, so the engine does not retain this
        /// identifier after applying the transaction.
        tx: TransactionId,
        /// Positive amount with at most four decimal places.
        amount: Decimal,
    },
    /// Places the funds from a successful deposit on hold.
    Dispute {
        /// Account that owns the referenced deposit.
        client: ClientId,
        /// Identifier of the referenced deposit.
        tx: TransactionId,
    },
    /// Returns held funds from an active dispute to the available balance.
    Resolve {
        /// Account that owns the referenced deposit.
        client: ClientId,
        /// Identifier of the referenced deposit.
        tx: TransactionId,
    },
    /// Removes held funds from an active dispute and locks the account.
    Chargeback {
        /// Account that owns the referenced deposit.
        client: ClientId,
        /// Identifier of the referenced deposit.
        tx: TransactionId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Result of a valid transaction request.
pub enum ApplyOutcome {
    /// The transaction changed ledger state.
    Applied,
    /// The requested balance or dispute transition did not apply.
    ///
    /// Some ignored requests can still establish an empty account, as described
    /// by [`IgnoreReason::InsufficientFunds`].
    Ignored(IgnoreReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Reason that a requested balance or dispute transition did not apply.
pub enum IgnoreReason {
    /// The withdrawal exceeded the available balance.
    ///
    /// If the client has no account yet, the withdrawal establishes an empty one.
    InsufficientFunds,
    /// No disputable deposit has the referenced transaction identifier.
    UnknownTransaction,
    /// The control row named a client other than the deposit owner.
    ClientMismatch,
    /// The deposit was not in the state required by the control operation.
    InvalidState,
    /// A previous chargeback locked the account.
    AccountLocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Read only view of a client account after processing.
pub struct AccountSnapshot {
    /// Client account identifier.
    pub client: ClientId,
    /// Balance not held by active disputes; may be negative after a dispute.
    pub available: Decimal,
    /// Nonnegative funds held by active disputes.
    pub held: Decimal,
    /// Sum of available and held funds, computed rather than stored.
    ///
    /// The value may be negative after a chargeback removes disputed funds that
    /// were already withdrawn.
    pub total: Decimal,
    /// Whether a chargeback locked the account; later valid transactions are ignored.
    pub locked: bool,
}
