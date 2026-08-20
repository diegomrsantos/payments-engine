//! Transaction row deserialization and validation.

use super::ProcessError;
use crate::money::SCALE;
use crate::{ClientId, Engine, Transaction, TransactionId};
use csv::{ReaderBuilder, StringRecord, Trim};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::io::Read;

/// Required input columns, accepted in any order.
const EXPECTED_HEADERS: [&str; 4] = ["type", "client", "tx", "amount"];

/// Reads and applies rows sequentially, returning state only after valid input ends.
///
/// The reader never collects the complete input. The returned engine retains
/// account state and deposits that later controls may reference. Transaction
/// row errors retain the one based CSV record number, with the header counted
/// as the first record, so diagnostics identify the failing transaction.
pub(super) fn read_engine<R: Read>(reader: R) -> Result<Engine, ProcessError> {
    let mut csv = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(false)
        .from_reader(reader);

    validate_headers(csv.headers().map_err(ProcessError::ReadHeaders)?)?;

    let mut engine = Engine::new();
    for (index, result) in csv.deserialize::<RawTransaction>().enumerate() {
        let row = index + 2;
        let raw = result.map_err(|source| ProcessError::ReadRow { row, source })?;
        let transaction = raw.into_transaction(row)?;
        engine
            .apply(transaction)
            .map_err(|source| ProcessError::Apply { row, source })?;
    }
    Ok(engine)
}

/// Requires each expected header exactly once while allowing any column order.
///
/// Rows are deserialized by header name, so position has no semantic meaning.
fn validate_headers(headers: &StringRecord) -> Result<(), ProcessError> {
    let valid = headers.len() == EXPECTED_HEADERS.len()
        && EXPECTED_HEADERS
            .iter()
            .all(|expected| headers.iter().any(|header| header == *expected));

    if valid {
        return Ok(());
    }

    Err(ProcessError::InvalidHeaders {
        found: headers.iter().map(str::to_owned).collect(),
    })
}

/// CSV row retained until rules that depend on transaction type are checked.
///
/// The amount remains text so empty values, plain decimal syntax, and written
/// precision can be validated before constructing a domain transaction.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTransaction {
    #[serde(rename = "type")]
    kind: String,
    client: ClientId,
    tx: TransactionId,
    amount: String,
}

impl RawTransaction {
    /// Validates row shape and converts textual fields into a typed transaction.
    ///
    /// Primary transactions require an amount. Control transactions require an
    /// empty amount so unexpected data is not silently discarded.
    fn into_transaction(self, row: usize) -> Result<Transaction, ProcessError> {
        let Self {
            kind,
            client,
            tx,
            amount,
        } = self;

        match kind.as_str() {
            "deposit" => Ok(Transaction::Deposit {
                client,
                tx,
                amount: required_amount(row, amount)?,
            }),
            "withdrawal" => Ok(Transaction::Withdrawal {
                client,
                tx,
                amount: required_amount(row, amount)?,
            }),
            "dispute" => {
                reject_amount(row, amount)?;
                Ok(Transaction::Dispute { client, tx })
            }
            "resolve" => {
                reject_amount(row, amount)?;
                Ok(Transaction::Resolve { client, tx })
            }
            "chargeback" => {
                reject_amount(row, amount)?;
                Ok(Transaction::Chargeback { client, tx })
            }
            unknown => Err(ProcessError::InvalidRow {
                row,
                message: format!("unknown transaction type {unknown:?}"),
            }),
        }
    }
}

/// Parses a required amount without allowing decimal rounding.
fn required_amount(row: usize, amount: String) -> Result<Decimal, ProcessError> {
    if amount.is_empty() {
        return Err(ProcessError::InvalidRow {
            row,
            message: "deposit and withdrawal rows require an amount".to_owned(),
        });
    }

    let normalized = normalized_decimal(row, &amount)?;
    Decimal::from_str_exact(&normalized).map_err(|_| ProcessError::InvalidRow {
        row,
        message: "amount must be an exact decimal number".to_owned(),
    })
}

/// Validates plain decimal syntax and returns an exactly equivalent parsing form.
///
/// Scientific notation is rejected. Input may contain at most [`SCALE`]
/// fractional digits, checked before trailing zeroes are removed. The normalized
/// form lets large equivalent values fit the decimal mantissa and inserts a zero
/// before fractions such as `.1250`.
fn normalized_decimal(row: usize, amount: &str) -> Result<String, ProcessError> {
    let (sign, unsigned) = if let Some(unsigned) = amount.strip_prefix('+') {
        ("+", unsigned)
    } else if let Some(unsigned) = amount.strip_prefix('-') {
        ("-", unsigned)
    } else {
        ("", amount)
    };

    let Some((whole, fraction)) = unsigned.split_once('.') else {
        if unsigned.is_empty() || !unsigned.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid_decimal(row));
        }
        return Ok(amount.to_owned());
    };

    let has_digit = !whole.is_empty() || !fraction.is_empty();
    let contains_only_digits = whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit());
    if !has_digit || !contains_only_digits {
        return Err(invalid_decimal(row));
    }
    if fraction.len() > SCALE as usize {
        return Err(ProcessError::InvalidRow {
            row,
            message: format!("amount may have at most {SCALE} decimal places"),
        });
    }

    let whole = if whole.is_empty() { "0" } else { whole };
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        Ok(format!("{sign}{whole}"))
    } else {
        Ok(format!("{sign}{whole}.{fraction}"))
    }
}

/// Creates the consistent row error used for malformed decimal text.
fn invalid_decimal(row: usize) -> ProcessError {
    ProcessError::InvalidRow {
        row,
        message: "amount must be an exact decimal number".to_owned(),
    }
}

/// Rejects data in the amount column of a control transaction.
fn reject_amount(row: usize, amount: String) -> Result<(), ProcessError> {
    if !amount.is_empty() {
        return Err(ProcessError::InvalidRow {
            row,
            message: "dispute, resolve, and chargeback rows must not contain an amount".to_owned(),
        });
    }
    Ok(())
}
