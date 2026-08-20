//! Balance arithmetic and account invariants.

use crate::money::Money;
use crate::{AccountSnapshot, ClientId, EngineError};

/// Balance and lock state for one client, without the identifier owned by the engine.
///
/// Available funds may become negative when a deposit is disputed after some
/// of its funds were withdrawn. Held funds are never negative. Every balance
/// update is validated before it replaces the stored values, so a failed
/// operation leaves the account intact.
#[derive(Debug, Default)]
pub(crate) struct Account {
    available: Money,
    held: Money,
    locked: bool,
}

impl Account {
    /// Returns the lock flag that the engine uses to reject later transactions.
    pub(crate) fn is_locked(&self) -> bool {
        self.locked
    }

    /// Credits available funds with exact arithmetic.
    ///
    /// The account remains unchanged if the resulting balance cannot be
    /// represented exactly.
    pub(crate) fn deposit(&mut self, client: ClientId, amount: Money) -> Result<(), EngineError> {
        let available = self.available.checked_add(amount, client)?;
        self.replace_balances(client, available, self.held)
    }

    /// Tries to debit available funds.
    ///
    /// Returns `false` without changing the account when funds are insufficient.
    /// Withdrawing the exact available balance succeeds. An arithmetic or
    /// invariant error also leaves the account unchanged.
    pub(crate) fn withdraw(
        &mut self,
        client: ClientId,
        amount: Money,
    ) -> Result<bool, EngineError> {
        if self.available < amount {
            return Ok(false);
        }

        let available = self.available.checked_sub(amount, client)?;
        self.replace_balances(client, available, self.held)?;
        Ok(true)
    }

    /// Moves the complete amount from available to held funds.
    ///
    /// Available funds may become negative when deposited money was already
    /// withdrawn. Held funds remain subject to the nonnegative invariant.
    pub(crate) fn hold(&mut self, client: ClientId, amount: Money) -> Result<(), EngineError> {
        let available = self.available.checked_sub(amount, client)?;
        let held = self.held.checked_add(amount, client)?;
        self.replace_balances(client, available, held)
    }

    /// Returns held funds to the available balance.
    ///
    /// Releasing more than the held balance is an invariant violation, and the
    /// account remains unchanged.
    pub(crate) fn release(&mut self, client: ClientId, amount: Money) -> Result<(), EngineError> {
        let available = self.available.checked_add(amount, client)?;
        let held = self.held.checked_sub(amount, client)?;
        self.replace_balances(client, available, held)
    }

    /// Removes held funds and locks the account after validation succeeds.
    ///
    /// The account remains unlocked if the balance change fails.
    pub(crate) fn chargeback(
        &mut self,
        client: ClientId,
        amount: Money,
    ) -> Result<(), EngineError> {
        let held = self.held.checked_sub(amount, client)?;
        self.replace_balances(client, self.available, held)?;
        self.locked = true;
        Ok(())
    }

    /// Creates an exact read only view with `total` derived rather than stored.
    pub(crate) fn snapshot(&self, client: ClientId) -> Result<AccountSnapshot, EngineError> {
        Ok(AccountSnapshot {
            client,
            available: self.available.to_decimal(client)?,
            held: self.held.to_decimal(client)?,
            total: self
                .available
                .checked_add(self.held, client)?
                .to_decimal(client)?,
            locked: self.locked,
        })
    }

    /// Validates both candidate balances before replacing either stored value.
    ///
    /// This is the single mutation point that enforces nonnegative held funds
    /// and exact representability of the derived total.
    fn replace_balances(
        &mut self,
        client: ClientId,
        available: Money,
        held: Money,
    ) -> Result<(), EngineError> {
        if held.is_negative() {
            return Err(EngineError::InvariantViolation { client });
        }
        available.checked_add(held, client)?;
        self.available = available;
        self.held = held;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
