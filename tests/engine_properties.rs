use payments_engine::{AccountSnapshot, Engine, Transaction};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::thread;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).expect("test decimal should be valid")
}

#[test]
fn a_complete_dispute_story_preserves_the_total_invariant_after_each_step() {
    let story = [
        Transaction::Deposit {
            client: 1,
            tx: 100,
            amount: decimal("5.0000"),
        },
        Transaction::Withdrawal {
            client: 1,
            tx: 101,
            amount: decimal("1.2500"),
        },
        Transaction::Dispute { client: 1, tx: 100 },
        Transaction::Resolve { client: 1, tx: 100 },
        Transaction::Dispute { client: 1, tx: 100 },
        Transaction::Chargeback { client: 1, tx: 100 },
    ];
    let mut engine = Engine::new();

    for transaction in story {
        engine
            .apply(transaction)
            .expect("story transaction should be valid");

        for account in engine.accounts().expect("accounts should be valid") {
            assert_eq!(
                account.available + account.held,
                account.total,
                "client {} total must equal available plus held",
                account.client
            );
        }
    }
}

#[test]
fn fractional_deposits_and_withdrawals_produce_an_exact_balance() {
    let mut engine = Engine::new();
    engine
        .apply(Transaction::Deposit {
            client: 4,
            tx: 300,
            amount: decimal("0.1001"),
        })
        .expect("first deposit should be valid");
    engine
        .apply(Transaction::Deposit {
            client: 4,
            tx: 301,
            amount: decimal("0.2002"),
        })
        .expect("second deposit should be valid");
    engine
        .apply(Transaction::Withdrawal {
            client: 4,
            tx: 302,
            amount: decimal("0.0003"),
        })
        .expect("withdrawal should be valid");

    let account = &engine.accounts().expect("accounts should be valid")[0];

    assert_eq!(account.available, decimal("0.3000"));
    assert_eq!(account.total, decimal("0.3000"));
}

#[test]
fn separate_engines_are_send_and_keep_state_isolated_across_threads() {
    // This assertion is checked by the compiler and documents that an Engine
    // owns all state needed to move between worker threads.
    assert_send::<Engine>();

    let first = thread::spawn(|| engine_with_deposit(1, 100, "2.5000"));
    let second = thread::spawn(|| engine_with_deposit(2, 200, "7.7500"));

    let first = first.join().expect("first worker should finish");
    let second = second.join().expect("second worker should finish");

    assert_eq!(
        first.accounts(),
        Ok(vec![AccountSnapshot {
            client: 1,
            available: decimal("2.5000"),
            held: Decimal::ZERO,
            total: decimal("2.5000"),
            locked: false,
        }])
    );
    assert_eq!(
        second.accounts(),
        Ok(vec![AccountSnapshot {
            client: 2,
            available: decimal("7.7500"),
            held: Decimal::ZERO,
            total: decimal("7.7500"),
            locked: false,
        }])
    );
}

fn assert_send<T: Send>() {}

fn engine_with_deposit(client: u16, tx: u32, amount: &str) -> Engine {
    let mut engine = Engine::new();
    engine
        .apply(Transaction::Deposit {
            client,
            tx,
            amount: decimal(amount),
        })
        .expect("worker deposit should be valid");
    engine
}
