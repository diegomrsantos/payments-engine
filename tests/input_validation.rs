use payments_engine::{EngineError, ProcessError, process_csv};

fn rejected_without_output(input: &str) -> ProcessError {
    let mut output = Vec::new();
    let error = process_csv(input.as_bytes(), &mut output).expect_err("input should be rejected");
    assert!(
        output.is_empty(),
        "rejected input must not produce account CSV"
    );
    error
}

#[test]
fn a_deposit_requires_an_amount() {
    let input = concat!("type,client,tx,amount\n", "deposit,1,100,\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
}

#[test]
fn a_withdrawal_requires_an_amount() {
    let input = concat!("type,client,tx,amount\n", "withdrawal,1,101,\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
}

#[test]
fn an_amount_must_be_an_exact_decimal_number() {
    let input = concat!("type,client,tx,amount\n", "deposit,1,102,not-a-number\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
}

#[test]
fn decimal_amounts_do_not_allow_digit_separators() {
    for amount in ["1_000", "1_0.5"] {
        let input = format!("type,client,tx,amount\ndeposit,1,102,{amount}\n");

        let error = rejected_without_output(&input);

        assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
    }
}

#[test]
fn an_amount_cannot_have_two_decimal_points() {
    let input = concat!("type,client,tx,amount\n", "deposit,1,103,1..0\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
}

#[test]
fn an_amount_must_be_positive() {
    let input = concat!("type,client,tx,amount\n", "deposit,1,100,.0000\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::Apply { row: 2, .. }));
    assert!(error.to_string().contains("amount must be positive"));
}

#[test]
fn an_amount_may_have_at_most_four_decimal_places() {
    let input = concat!("type,client,tx,amount\n", "deposit,1,100,0.00001\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
}

#[test]
fn a_large_overprecise_amount_is_rejected_without_rounding() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,1,100,10000000000000.00001\n",
    );

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidRow { row: 2, .. }));
}

#[test]
fn an_account_balance_that_cannot_grow_exactly_is_rejected() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,1,100,7922816251426433759354395.0335\n",
        "deposit,1,101,0.0001\n",
    );

    let error = rejected_without_output(input);

    assert!(matches!(
        error,
        ProcessError::Apply {
            row: 3,
            source: EngineError::ArithmeticOverflow { client: 1 },
        }
    ));
}

#[test]
fn identifiers_must_fit_their_declared_integer_types() {
    let input = concat!("type,client,tx,amount\n", "deposit,70000,100,1.0000\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::ReadRow { row: 2, .. }));
}

#[test]
fn the_input_requires_exactly_the_transaction_headers() {
    let input = concat!("type,client,transaction,amount\n", "deposit,1,100,1.0000\n",);

    let error = rejected_without_output(input);

    assert!(matches!(error, ProcessError::InvalidHeaders { .. }));
}
