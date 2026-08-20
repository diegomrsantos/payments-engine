//! Exact fixed scale monetary arithmetic.

use crate::{ClientId, EngineError};
use rust_decimal::Decimal;

/// Number of decimal places represented by one internal monetary unit.
pub(crate) const SCALE: u32 = 4;

/// Exact signed count of units worth `10^-4` each.
///
/// Signed storage permits disputes to make available funds negative. Values
/// must also have an exact [`Decimal`] representation so public snapshots can
/// never require rounding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Money {
    minor_units: i128,
}

impl Money {
    /// Converts a positive transaction amount with at most four decimal places.
    ///
    /// The conversion multiplies the mantissa into fixed scale units without
    /// rounding. It rejects a nonpositive amount, more than four decimal places,
    /// and values that cannot remain exactly representable.
    pub(crate) fn from_transaction_amount(
        client: ClientId,
        amount: Decimal,
    ) -> Result<Self, EngineError> {
        if amount <= Decimal::ZERO {
            return Err(EngineError::InvalidAmount { amount });
        }
        if amount.scale() > SCALE {
            return Err(EngineError::ExcessPrecision {
                amount,
                scale: amount.scale(),
            });
        }

        let scale_factor = 10_i128.pow(SCALE - amount.scale());
        let minor_units = amount
            .mantissa()
            .checked_mul(scale_factor)
            .ok_or(EngineError::ArithmeticOverflow { client })?;
        Self::from_minor_units(client, minor_units)
    }

    /// Adds two values without rounding or wrapping.
    ///
    /// Results outside either the `i128` range or the exact [`Decimal`] range
    /// are rejected.
    pub(crate) fn checked_add(self, other: Self, client: ClientId) -> Result<Self, EngineError> {
        let minor_units = self
            .minor_units
            .checked_add(other.minor_units)
            .ok_or(EngineError::ArithmeticOverflow { client })?;
        Self::from_minor_units(client, minor_units)
    }

    /// Subtracts two values without rounding or wrapping.
    ///
    /// Results outside either the `i128` range or the exact [`Decimal`] range
    /// are rejected.
    pub(crate) fn checked_sub(self, other: Self, client: ClientId) -> Result<Self, EngineError> {
        let minor_units = self
            .minor_units
            .checked_sub(other.minor_units)
            .ok_or(EngineError::ArithmeticOverflow { client })?;
        Self::from_minor_units(client, minor_units)
    }

    /// Converts the stored units to an exact decimal value.
    ///
    /// Failure indicates an invalid internal representation rather than bad
    /// transaction input.
    pub(crate) fn to_decimal(self, client: ClientId) -> Result<Decimal, EngineError> {
        exact_decimal(self.minor_units).ok_or(EngineError::InvariantViolation { client })
    }

    /// Reports whether the signed unit count is below zero.
    pub(crate) fn is_negative(self) -> bool {
        self.minor_units < 0
    }

    /// Central constructor that requires an exact public decimal representation.
    fn from_minor_units(client: ClientId, minor_units: i128) -> Result<Self, EngineError> {
        exact_decimal(minor_units).ok_or(EngineError::ArithmeticOverflow { client })?;
        Ok(Self { minor_units })
    }
}

/// Builds an exact decimal without rounding.
///
/// When fixed scale units exceed the decimal mantissa range, the conversion
/// removes trailing coefficient zeroes together with scale digits. This keeps
/// the value unchanged. It returns `None` when no exact representation exists.
fn exact_decimal(mut minor_units: i128) -> Option<Decimal> {
    let mut scale = SCALE;
    loop {
        if let Ok(decimal) = Decimal::try_from_i128_with_scale(minor_units, scale) {
            return Some(decimal);
        }
        if scale == 0 || minor_units % 10 != 0 {
            return None;
        }
        minor_units /= 10;
        scale -= 1;
    }
}

#[cfg(test)]
mod tests;
