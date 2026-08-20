use super::*;

mod transactions;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str_exact(value).expect("test decimal should be exact")
}

fn deposit(client: ClientId, tx: TransactionId, amount: &str) -> Transaction {
    Transaction::Deposit {
        client,
        tx,
        amount: decimal(amount),
    }
}

fn withdrawal(client: ClientId, tx: TransactionId, amount: &str) -> Transaction {
    Transaction::Withdrawal {
        client,
        tx,
        amount: decimal(amount),
    }
}

fn apply(engine: &mut Engine, transaction: Transaction) -> ApplyOutcome {
    engine
        .apply(transaction)
        .expect("scenario transaction should be valid")
}

fn account(engine: &Engine, client: ClientId) -> AccountSnapshot {
    engine
        .accounts()
        .expect("scenario accounts should be valid")
        .into_iter()
        .find(|account| account.client == client)
        .expect("scenario account should exist")
}
