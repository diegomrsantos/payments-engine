//! Transaction coordination for account balances.

use crate::account::Account;
use crate::money::Money;
use crate::{
    AccountSnapshot, ApplyOutcome, ClientId, EngineError, IgnoreReason, Transaction, TransactionId,
};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
/// Ledger state for one chronological transaction stream.
pub struct Engine {
    accounts: HashMap<ClientId, Account>,
    deposits: HashSet<TransactionId>,
}

impl Engine {
    /// Creates an empty engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one transaction to the current ledger state.
    ///
    /// # Errors
    ///
    /// Returns an error when an amount or deposit identifier is invalid, exact
    /// balance arithmetic cannot be completed, or an invariant is violated.
    pub fn apply(&mut self, transaction: Transaction) -> Result<ApplyOutcome, EngineError> {
        match transaction {
            Transaction::Deposit { client, tx, amount } => self.deposit(client, tx, amount),
            Transaction::Withdrawal { client, amount, .. } => self.withdraw(client, amount),
        }
    }

    /// Returns account snapshots in ascending client order.
    ///
    /// # Errors
    ///
    /// Returns an error if internal balances cannot be represented exactly.
    pub fn accounts(&self) -> Result<Vec<AccountSnapshot>, EngineError> {
        let mut snapshots = self
            .accounts
            .iter()
            .map(|(&client, account)| account.snapshot(client))
            .collect::<Result<Vec<_>, _>>()?;
        snapshots.sort_unstable_by_key(|account| account.client);
        Ok(snapshots)
    }

    fn deposit(
        &mut self,
        client: ClientId,
        tx: TransactionId,
        amount: Decimal,
    ) -> Result<ApplyOutcome, EngineError> {
        let amount = Money::from_transaction_amount(client, amount)?;
        if self.deposits.contains(&tx) {
            return Err(EngineError::DuplicateTransaction { tx });
        }

        self.accounts
            .entry(client)
            .or_default()
            .deposit(client, amount)?;
        self.deposits.insert(tx);
        Ok(ApplyOutcome::Applied)
    }

    fn withdraw(&mut self, client: ClientId, amount: Decimal) -> Result<ApplyOutcome, EngineError> {
        let amount = Money::from_transaction_amount(client, amount)?;
        let account = self.accounts.entry(client).or_default();
        if !account.withdraw(client, amount)? {
            return Ok(ApplyOutcome::Ignored(IgnoreReason::InsufficientFunds));
        }
        Ok(ApplyOutcome::Applied)
    }
}

#[cfg(test)]
mod tests;
