//! Balance arithmetic and account invariants.

use crate::money::Money;
use crate::{AccountSnapshot, ClientId, EngineError};

/// Balance state for one client, without the identifier owned by the engine.
///
/// Every balance update is validated before it replaces the stored values, so
/// a failed operation leaves the account intact.
#[derive(Debug, Default)]
pub(crate) struct Account {
    available: Money,
    held: Money,
}

impl Account {
    /// Credits available funds with exact arithmetic.
    pub(crate) fn deposit(&mut self, client: ClientId, amount: Money) -> Result<(), EngineError> {
        let available = self.available.checked_add(amount, client)?;
        self.replace_balances(client, available, self.held)
    }

    /// Tries to debit available funds.
    ///
    /// Returns `false` without changing the account when funds are insufficient.
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
            locked: false,
        })
    }

    /// Validates both candidate balances before replacing either stored value.
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
