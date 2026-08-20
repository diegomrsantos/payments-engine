use super::*;

#[test]
fn a_deposit_then_withdrawal_leaves_the_remaining_funds_available() {
    let mut engine = Engine::new();

    assert_eq!(
        apply(&mut engine, deposit(12, 901, "4.1250")),
        ApplyOutcome::Applied
    );
    assert_eq!(
        apply(&mut engine, withdrawal(12, 902, "1.0250")),
        ApplyOutcome::Applied
    );

    assert_eq!(
        account(&engine, 12),
        AccountSnapshot {
            client: 12,
            available: decimal("3.1000"),
            held: Decimal::ZERO,
            total: decimal("3.1000"),
            locked: false,
        }
    );
}

#[test]
fn withdrawing_the_complete_available_balance_succeeds() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(12, 903, "4.1250"));

    let outcome = apply(&mut engine, withdrawal(12, 904, "4.1250"));

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(
        account(&engine, 12),
        AccountSnapshot {
            client: 12,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    );
}

#[test]
fn an_insufficient_withdrawal_is_ignored_without_changing_the_balance() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(4, 100, "1.0000"));

    let outcome = apply(&mut engine, withdrawal(4, 101, "1.0001"));

    assert_eq!(
        outcome,
        ApplyOutcome::Ignored(IgnoreReason::InsufficientFunds)
    );
    assert_eq!(account(&engine, 4).available, decimal("1.0000"));
}

#[test]
fn a_first_withdrawal_creates_an_account_even_when_it_is_declined() {
    let mut engine = Engine::new();

    let outcome = apply(&mut engine, withdrawal(6, 200, "2.0000"));

    assert_eq!(
        outcome,
        ApplyOutcome::Ignored(IgnoreReason::InsufficientFunds)
    );
    assert_eq!(account(&engine, 6).total, Decimal::ZERO);
}

#[test]
fn account_snapshots_are_sorted_by_client() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(20, 300, "1.0000"));
    apply(&mut engine, deposit(3, 301, "1.0000"));
    apply(&mut engine, deposit(11, 302, "1.0000"));

    let clients = engine
        .accounts()
        .expect("accounts should be valid")
        .into_iter()
        .map(|account| account.client)
        .collect::<Vec<_>>();

    assert_eq!(clients, vec![3, 11, 20]);
}

#[test]
fn invalid_deposit_amount_does_not_create_an_account() {
    let mut engine = Engine::new();

    let result = engine.apply(deposit(8, 400, "0.00001"));

    assert!(matches!(result, Err(EngineError::ExcessPrecision { .. })));
    assert!(
        engine
            .accounts()
            .expect("accounts should be valid")
            .is_empty()
    );
}

#[test]
fn duplicate_deposit_ids_are_rejected_without_changing_balances() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(1, 500, "2.0000"));

    let result = engine.apply(deposit(2, 500, "8.0000"));

    assert_eq!(result, Err(EngineError::DuplicateTransaction { tx: 500 }));
    assert_eq!(account(&engine, 1).total, decimal("2.0000"));
    assert!(
        engine
            .accounts()
            .expect("accounts should be valid")
            .into_iter()
            .all(|account| account.client != 2)
    );
}

#[test]
fn an_inexact_deposit_is_rejected_without_changing_state() {
    let mut engine = Engine::new();
    apply(
        &mut engine,
        deposit(1, 600, "792281625142643375935439503.5"),
    );
    let before = account(&engine, 1);

    let result = engine.apply(deposit(1, 601, "0.0001"));

    assert_eq!(result, Err(EngineError::ArithmeticOverflow { client: 1 }));
    assert_eq!(account(&engine, 1), before);
    assert_eq!(
        apply(&mut engine, dispute(1, 601)),
        ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction)
    );
}
