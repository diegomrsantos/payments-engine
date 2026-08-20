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

    assert!(
        !account
            .withdraw(8, money("0.5000"))
            .expect("withdrawal should be valid")
    );
    assert_eq!(
        account.snapshot(8).expect("snapshot should be valid").total,
        Decimal::ZERO
    );
}
