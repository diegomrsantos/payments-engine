use super::*;

#[test]
fn disputing_a_deposit_moves_its_funds_from_available_to_held() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(7, 600, "3.2500"));

    let outcome = apply(&mut engine, dispute(7, 600));

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(
        account(&engine, 7),
        AccountSnapshot {
            client: 7,
            available: Decimal::ZERO,
            held: decimal("3.2500"),
            total: decimal("3.2500"),
            locked: false,
        }
    );
}

#[test]
fn resolving_a_dispute_returns_held_funds_to_available() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(7, 610, "3.2500"));
    apply(&mut engine, dispute(7, 610));

    let outcome = apply(&mut engine, resolve(7, 610));

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(account(&engine, 7).available, decimal("3.2500"));
    assert_eq!(account(&engine, 7).held, Decimal::ZERO);
}

#[test]
fn a_resolved_deposit_can_be_disputed_again() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(7, 620, "3.2500"));
    apply(&mut engine, dispute(7, 620));
    apply(&mut engine, resolve(7, 620));

    let outcome = apply(&mut engine, dispute(7, 620));

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(account(&engine, 7).held, decimal("3.2500"));
}

#[test]
fn charging_back_a_dispute_removes_held_funds_and_locks_the_account() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(9, 630, "5.0000"));
    apply(&mut engine, dispute(9, 630));

    let outcome = apply(&mut engine, chargeback(9, 630));

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(
        account(&engine, 9),
        AccountSnapshot {
            client: 9,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: true,
        }
    );
}

#[test]
fn a_chargeback_locks_the_account_with_another_dispute_still_held() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(9, 631, "5.0000"));
    apply(&mut engine, deposit(9, 632, "2.5000"));
    apply(&mut engine, dispute(9, 631));
    apply(&mut engine, dispute(9, 632));

    let outcome = apply(&mut engine, chargeback(9, 631));

    assert_eq!(outcome, ApplyOutcome::Applied);
    let locked_account = AccountSnapshot {
        client: 9,
        available: Decimal::ZERO,
        held: decimal("2.5000"),
        total: decimal("2.5000"),
        locked: true,
    };
    assert_eq!(account(&engine, 9), locked_account);

    let resolve_other = apply(&mut engine, resolve(9, 632));
    let chargeback_other = apply(&mut engine, chargeback(9, 632));

    assert_eq!(
        resolve_other,
        ApplyOutcome::Ignored(IgnoreReason::AccountLocked)
    );
    assert_eq!(
        chargeback_other,
        ApplyOutcome::Ignored(IgnoreReason::AccountLocked)
    );
    assert_eq!(account(&engine, 9), locked_account);
}

#[test]
fn a_dispute_can_make_available_funds_negative_after_money_was_spent() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(2, 640, "5.0000"));
    apply(&mut engine, withdrawal(2, 641, "4.2500"));

    apply(&mut engine, dispute(2, 640));

    assert_eq!(account(&engine, 2).available, decimal("-4.2500"));
    assert_eq!(account(&engine, 2).held, decimal("5.0000"));
    assert_eq!(account(&engine, 2).total, decimal("0.7500"));
}

#[test]
fn controls_for_an_unknown_deposit_are_ignored_without_creating_an_account() {
    let mut engine = Engine::new();

    let dispute = apply(&mut engine, dispute(3, 999));
    let resolve = apply(&mut engine, resolve(3, 999));
    let chargeback = apply(&mut engine, chargeback(3, 999));

    assert_eq!(
        dispute,
        ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction)
    );
    assert_eq!(
        resolve,
        ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction)
    );
    assert_eq!(
        chargeback,
        ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction)
    );
    assert!(
        engine
            .accounts()
            .expect("accounts should be valid")
            .is_empty()
    );
}

#[test]
fn controls_for_another_client_are_ignored_without_creating_an_account() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(3, 652, "2.0000"));

    let mismatched_dispute = apply(&mut engine, dispute(4, 652));
    apply(&mut engine, dispute(3, 652));
    let resolve = apply(&mut engine, resolve(4, 652));
    let chargeback = apply(&mut engine, chargeback(4, 652));

    assert_eq!(
        mismatched_dispute,
        ApplyOutcome::Ignored(IgnoreReason::ClientMismatch)
    );
    assert_eq!(resolve, ApplyOutcome::Ignored(IgnoreReason::ClientMismatch));
    assert_eq!(
        chargeback,
        ApplyOutcome::Ignored(IgnoreReason::ClientMismatch)
    );
    assert_eq!(account(&engine, 3).available, Decimal::ZERO);
    assert_eq!(account(&engine, 3).held, decimal("2.0000"));
    assert_eq!(
        engine.accounts().expect("accounts should be valid").len(),
        1
    );
}

#[test]
fn a_successful_withdrawal_is_not_disputable() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(3, 655, "5.0000"));
    apply(&mut engine, withdrawal(3, 656, "2.0000"));

    let outcome = apply(&mut engine, dispute(3, 656));

    assert_eq!(
        outcome,
        ApplyOutcome::Ignored(IgnoreReason::UnknownTransaction)
    );
    assert_eq!(account(&engine, 3).available, decimal("3.0000"));
    assert_eq!(account(&engine, 3).held, Decimal::ZERO);
}

#[test]
fn controls_that_do_not_match_the_current_dispute_state_are_ignored() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(5, 660, "2.0000"));

    let resolve = apply(&mut engine, resolve(5, 660));
    let chargeback = apply(&mut engine, chargeback(5, 660));
    apply(&mut engine, dispute(5, 660));
    let repeated_dispute = apply(&mut engine, dispute(5, 660));

    assert_eq!(resolve, ApplyOutcome::Ignored(IgnoreReason::InvalidState));
    assert_eq!(
        chargeback,
        ApplyOutcome::Ignored(IgnoreReason::InvalidState)
    );
    assert_eq!(
        repeated_dispute,
        ApplyOutcome::Ignored(IgnoreReason::InvalidState)
    );
    assert_eq!(
        account(&engine, 5),
        AccountSnapshot {
            client: 5,
            available: Decimal::ZERO,
            held: decimal("2.0000"),
            total: decimal("2.0000"),
            locked: false,
        }
    );
}

#[test]
fn every_later_transaction_for_a_charged_back_account_is_ignored() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(10, 670, "4.0000"));
    apply(&mut engine, dispute(10, 670));
    apply(&mut engine, chargeback(10, 670));

    let deposit = apply(&mut engine, deposit(10, 671, "1.0000"));
    let withdrawal = apply(&mut engine, withdrawal(10, 672, "1.0000"));
    let dispute = apply(&mut engine, dispute(10, 670));
    let resolve = apply(&mut engine, resolve(10, 670));
    let chargeback = apply(&mut engine, chargeback(10, 670));

    assert_eq!(deposit, ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
    assert_eq!(
        withdrawal,
        ApplyOutcome::Ignored(IgnoreReason::AccountLocked)
    );
    assert_eq!(dispute, ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
    assert_eq!(resolve, ApplyOutcome::Ignored(IgnoreReason::AccountLocked));
    assert_eq!(
        chargeback,
        ApplyOutcome::Ignored(IgnoreReason::AccountLocked)
    );
    assert_eq!(account(&engine, 10).total, Decimal::ZERO);
}

#[test]
fn a_locked_account_does_not_stop_another_client_from_processing() {
    let mut engine = Engine::new();
    apply(&mut engine, deposit(10, 680, "4.0000"));
    apply(&mut engine, dispute(10, 680));
    apply(&mut engine, chargeback(10, 680));

    let outcome = apply(&mut engine, deposit(11, 681, "6.0000"));

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(account(&engine, 11).available, decimal("6.0000"));
}
