use super::*;
use crate::money::Money;
use rust_decimal::Decimal;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str_exact(value).expect("test decimal should be exact")
}

fn money(value: &str) -> Money {
    Money::from_transaction_amount(1, decimal(value)).expect("test amount should be valid")
}

#[test]
fn deposits_and_withdrawals_preserve_exact_balances() {
    let mut account = Account::default();

    account
        .deposit(3, money("4.1250"))
        .expect("deposit should succeed");
    assert!(
        account
            .withdraw(3, money("1.0250"))
            .expect("withdrawal should be valid")
    );

    assert_eq!(
        account.snapshot(3),
        Ok(AccountSnapshot {
            client: 3,
            available: decimal("3.1000"),
            held: Decimal::ZERO,
            total: decimal("3.1000"),
            locked: false,
        })
    );
}

#[test]
fn insufficient_withdrawal_does_not_change_the_account() {
    let mut account = Account::default();
    let before = account.snapshot(8).expect("snapshot should be valid");

    let withdrawn = account
        .withdraw(8, money("0.5000"))
        .expect("withdrawal should be valid");

    assert!(!withdrawn);
    assert_eq!(account.snapshot(8), Ok(before));
}

#[test]
fn held_funds_move_without_changing_total() {
    let mut account = Account::default();
    account
        .deposit(5, money("3.2500"))
        .expect("deposit should succeed");
    let settled = account.snapshot(5).expect("snapshot should be valid");

    account
        .hold(5, money("3.2500"))
        .expect("hold should succeed");
    assert_eq!(
        account.snapshot(5),
        Ok(AccountSnapshot {
            client: 5,
            available: Decimal::ZERO,
            held: decimal("3.2500"),
            total: decimal("3.2500"),
            locked: false,
        })
    );

    account
        .release(5, money("3.2500"))
        .expect("release should succeed");
    assert_eq!(account.snapshot(5), Ok(settled));
}

#[test]
fn releasing_more_than_the_held_balance_is_rejected_atomically() {
    let mut account = Account::default();
    account
        .deposit(5, money("2.0000"))
        .expect("deposit should succeed");
    account
        .hold(5, money("1.0000"))
        .expect("hold should succeed");
    let before = account.snapshot(5).expect("snapshot should be valid");

    let result = account.release(5, money("2.0000"));

    assert_eq!(result, Err(EngineError::InvariantViolation { client: 5 }));
    assert_eq!(
        account.snapshot(5).expect("snapshot should be valid"),
        before
    );
}

#[test]
fn chargeback_can_leave_a_negative_total_and_locks_the_account() {
    let mut account = Account::default();
    account
        .deposit(9, money("5.0000"))
        .expect("deposit should succeed");
    account
        .withdraw(9, money("4.2500"))
        .expect("withdrawal should succeed");
    account
        .hold(9, money("5.0000"))
        .expect("hold should succeed");

    account
        .chargeback(9, money("5.0000"))
        .expect("chargeback should succeed");

    assert_eq!(
        account.snapshot(9),
        Ok(AccountSnapshot {
            client: 9,
            available: decimal("-4.2500"),
            held: Decimal::ZERO,
            total: decimal("-4.2500"),
            locked: true,
        })
    );
}
