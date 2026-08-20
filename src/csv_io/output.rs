//! Account row serialization.

use super::ProcessError;
use crate::EngineError;
use crate::money::SCALE;
use crate::{AccountSnapshot, ClientId};
use serde::Serialize;
use std::io::Write;

/// Formats every account before writing the CSV document.
///
/// Preparing all rows first prevents a formatting invariant failure from
/// leaving plausible partial account data in the destination.
pub(super) fn write_accounts<W: Write>(
    writer: W,
    accounts: Vec<AccountSnapshot>,
) -> Result<(), ProcessError> {
    let rows = accounts
        .into_iter()
        .map(AccountRow::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ProcessError::Finalize)?;
    let mut csv = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(writer);
    csv.write_record(["client", "available", "held", "total", "locked"])
        .map_err(ProcessError::Write)?;
    for row in rows {
        csv.serialize(row).map_err(ProcessError::Write)?;
    }
    csv.flush().map_err(ProcessError::Flush)
}

/// Serialization shape with balances already formatted to four decimal places.
#[derive(Debug, Serialize)]
struct AccountRow {
    client: ClientId,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl TryFrom<AccountSnapshot> for AccountRow {
    type Error = EngineError;

    /// Converts every balance through the same exact fixed scale formatter.
    fn try_from(account: AccountSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            client: account.client,
            available: format_decimal(account.available, account.client)?,
            held: format_decimal(account.held, account.client)?,
            total: format_decimal(account.total, account.client)?,
            locked: account.locked,
        })
    }
}

/// Formats an exact ledger value with four decimal places and no rounding.
///
/// Integer decomposition handles the complete supported range, including
/// negative values. A scale above the ledger limit is an invariant violation.
fn format_decimal(value: rust_decimal::Decimal, client: ClientId) -> Result<String, EngineError> {
    if value.scale() > SCALE {
        return Err(EngineError::InvariantViolation { client });
    }

    let scale_factor = 10_u128.pow(SCALE - value.scale());
    let minor_units = value
        .mantissa()
        .unsigned_abs()
        .checked_mul(scale_factor)
        .ok_or(EngineError::InvariantViolation { client })?;
    let whole = minor_units / 10_u128.pow(SCALE);
    let fraction = minor_units % 10_u128.pow(SCALE);
    let sign = if value.is_sign_negative() && minor_units != 0 {
        "-"
    } else {
        ""
    };

    Ok(format!("{sign}{whole}.{fraction:04}"))
}
