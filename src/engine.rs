//! Transaction coordination and deposit dispute state.

use crate::account::Account;
use crate::money::Money;
use crate::{
    AccountSnapshot, ApplyOutcome, ClientId, EngineError, IgnoreReason, Transaction, TransactionId,
};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Default)]
/// Ledger state for one chronological transaction stream.
///
/// An engine owns all of its state. Separate instances can therefore be moved
/// to different worker threads without shared mutable state.
pub struct Engine {
    accounts: HashMap<ClientId, Account>,
    deposits: HashMap<TransactionId, DepositRecord>,
}

impl Engine {
    /// Creates an empty engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one transaction to the current ledger state.
    ///
    /// Domain operations that cannot apply, such as an insufficient
    /// withdrawal or an unknown dispute reference, return an ignored outcome.
    ///
    /// # Errors
    ///
    /// Returns an error when an amount is invalid, a deposit identifier is
    /// duplicated, exact balance arithmetic cannot be completed, or internal
    /// ledger state violates an invariant.
    pub fn apply(&mut self, transaction: Transaction) -> Result<ApplyOutcome, EngineError> {
        match transaction {
            Transaction::Deposit { client, tx, amount } => self.deposit(client, tx, amount),
            Transaction::Withdrawal { client, amount, .. } => self.withdraw(client, amount),
            Transaction::Dispute { client, tx } => self.dispute(client, tx),
            Transaction::Resolve { client, tx } => self.resolve(client, tx),
            Transaction::Chargeback { client, tx } => self.chargeback(client, tx),
        }
    }

    /// Returns account snapshots in ascending client order.
    ///
    /// # Errors
    ///
    /// Returns an error if internal balances or a derived total cannot be
    /// represented exactly.
    pub fn accounts(&self) -> Result<Vec<AccountSnapshot>, EngineError> {
        let mut snapshots = self
            .accounts
            .iter()
            .map(|(&client, account)| account.snapshot(client))
            .collect::<Result<Vec<_>, _>>()?;
        snapshots.sort_unstable_by_key(|account| account.client);
        Ok(snapshots)
    }

    /// Applies a valid deposit and retains its metadata for later controls.
    ///
    /// A duplicate retained deposit identifier is an error. A locked account
    /// ignores an otherwise valid deposit. Only a deposit that changes the
    /// balance becomes a disputable record.
    fn deposit(
        &mut self,
        client: ClientId,
        tx: TransactionId,
        amount: Decimal,
    ) -> Result<ApplyOutcome, EngineError> {
        let amount = Money::from_transaction_amount(client, amount)?;
        if self.deposits.contains_key(&tx) {
            return Err(EngineError::DuplicateTransaction { tx });
        }

        let account = self.accounts.entry(client).or_default();
        if account.is_locked() {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
        }

        account.deposit(client, amount)?;
        self.deposits.insert(
            tx,
            DepositRecord {
                client,
                amount,
                state: DepositState::Settled,
            },
        );
        Ok(ApplyOutcome::Applied)
    }

    /// Applies a withdrawal or reports why it cannot change the account.
    ///
    /// A valid withdrawal establishes the client before lock and balance checks,
    /// even when insufficient funds leave a new account empty. Its transaction
    /// identifier is not retained because withdrawals cannot be disputed.
    fn withdraw(&mut self, client: ClientId, amount: Decimal) -> Result<ApplyOutcome, EngineError> {
        let amount = Money::from_transaction_amount(client, amount)?;
        let account = self.accounts.entry(client).or_default();
        if account.is_locked() {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
        }
        if !account.withdraw(client, amount)? {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::InsufficientFunds));
        }
        Ok(ApplyOutcome::Applied)
    }

    /// Moves a settled deposit from available to held funds.
    ///
    /// The referenced deposit must exist and belong to the named client.
    /// Unknown and mismatched references do not create accounts. Available
    /// funds may become negative when deposited money was already withdrawn.
    fn dispute(
        &mut self,
        client: ClientId,
        tx: TransactionId,
    ) -> Result<ApplyOutcome, EngineError> {
        let Some(record) = self.deposits.get(&tx).copied() else {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction));
        };
        if record.client != client {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::ClientMismatch));
        }

        let account = self.account_for_control(client)?;
        if account.is_locked() {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
        }
        if record.state != DepositState::Settled {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::InvalidState));
        }

        account.hold(client, record.amount)?;
        self.deposits
            .get_mut(&tx)
            .ok_or(EngineError::InvariantViolation { client })?
            .state = DepositState::Disputed;
        Ok(ApplyOutcome::Applied)
    }

    /// Returns an actively disputed deposit from held to available funds.
    ///
    /// The deposit returns to the settled state, which permits a later dispute.
    fn resolve(
        &mut self,
        client: ClientId,
        tx: TransactionId,
    ) -> Result<ApplyOutcome, EngineError> {
        let Some(record) = self.deposits.get(&tx).copied() else {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction));
        };
        if record.client != client {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::ClientMismatch));
        }

        let account = self.account_for_control(client)?;
        if account.is_locked() {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
        }
        if record.state != DepositState::Disputed {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::InvalidState));
        }

        account.release(client, record.amount)?;
        self.deposits
            .get_mut(&tx)
            .ok_or(EngineError::InvariantViolation { client })?
            .state = DepositState::Settled;
        Ok(ApplyOutcome::Applied)
    }

    /// Completes an active dispute by removing held funds and locking its account.
    ///
    /// The charged back state is terminal, and the engine ignores every later
    /// valid transaction for the owning client.
    fn chargeback(
        &mut self,
        client: ClientId,
        tx: TransactionId,
    ) -> Result<ApplyOutcome, EngineError> {
        let Some(record) = self.deposits.get(&tx).copied() else {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction));
        };
        if record.client != client {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::ClientMismatch));
        }

        let account = self.account_for_control(client)?;
        if account.is_locked() {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
        }
        if record.state != DepositState::Disputed {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::InvalidState));
        }

        account.chargeback(client, record.amount)?;
        self.deposits
            .get_mut(&tx)
            .ok_or(EngineError::InvariantViolation { client })?
            .state = DepositState::ChargedBack;
        Ok(ApplyOutcome::Applied)
    }

    /// Finds the account that must accompany retained deposit metadata.
    ///
    /// A missing account indicates corrupted internal state rather than an
    /// unknown control reference, which is handled before this lookup.
    fn account_for_control(&mut self, client: ClientId) -> Result<&mut Account, EngineError> {
        self.accounts
            .get_mut(&client)
            .ok_or(EngineError::InvariantViolation { client })
    }
}

/// Metadata retained for controls that may reference a successful deposit.
#[derive(Debug, Clone, Copy)]
struct DepositRecord {
    client: ClientId,
    amount: Money,
    state: DepositState,
}

/// Lifecycle of a successful deposit in the dispute state machine.
///
/// Valid transitions are `Settled -> Disputed -> ChargedBack` and
/// `Disputed -> Settled`. Returning to `Settled` permits another dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DepositState {
    /// The amount belongs to available funds and a new dispute may begin.
    Settled,
    /// The amount belongs to held funds and may be resolved or charged back.
    Disputed,
    /// The amount was removed and the owning account was locked.
    ChargedBack,
}

#[cfg(test)]
mod tests;
