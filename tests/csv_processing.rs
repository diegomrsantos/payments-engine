use payments_engine::{ProcessError, process_csv};

fn process(input: &str) -> Result<String, ProcessError> {
    let mut output = Vec::new();
    process_csv(input.as_bytes(), &mut output)?;
    Ok(String::from_utf8(output).expect("account CSV should be UTF-8"))
}

#[test]
fn interleaved_clients_are_processed_in_input_order_and_sorted_in_output() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,12,901,4.1250\n",
        "deposit,7,800,9.5000\n",
        "dispute,7,800,\n",
        "withdrawal,7,700,1.0000\n",
        "withdrawal,12,600,1.0250\n",
        "resolve,7,800,\n",
        "withdrawal,7,500,2.2500\n",
    );

    let output = process(input).expect("CSV story should be valid");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "7,7.2500,0.0000,7.2500,false\n",
            "12,3.1000,0.0000,3.1000,false\n",
        )
    );
}

#[test]
fn surrounding_whitespace_in_headers_and_fields_is_ignored() {
    let input = concat!(
        " type , client , tx , amount \n",
        " deposit , 12 , 901 , 4.1250 \n",
    );

    let output = process(input).expect("surrounding whitespace should be valid");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "12,4.1250,0.0000,4.1250,false\n",
        )
    );
}

#[test]
fn a_chargeback_locks_only_the_affected_account() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,2,100,5.0000\n",
        "withdrawal,2,101,4.0000\n",
        "dispute,2,100,\n",
        "chargeback,2,100,\n",
        "deposit,2,102,9.0000\n",
        "deposit,3,103,2.5000\n",
    );

    let output = process(input).expect("CSV story should be valid");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "2,-4.0000,0.0000,-4.0000,true\n",
            "3,2.5000,0.0000,2.5000,false\n",
        )
    );
}

#[test]
fn input_headers_may_appear_in_a_different_order() {
    let input = concat!("tx,amount,type,client\n", "700,1.2500,deposit,4\n",);

    let output = process(input).expect("reordered headers should be valid");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "4,1.2500,0.0000,1.2500,false\n",
        )
    );
}

#[test]
fn a_header_only_input_writes_a_header_only_output() {
    let output = process("type,client,tx,amount\n").expect("empty ledger should be valid");

    assert_eq!(output, "client,available,held,total,locked\n");
}

#[test]
fn supported_decimal_scales_and_maximum_identifiers_are_accepted() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,65535,4294967295,1\n",
        "deposit,65535,4294967294,0.1\n",
        "deposit,65535,4294967293,0.01\n",
        "deposit,65535,4294967292,0.001\n",
        "deposit,65535,4294967291,0.0001\n",
    );

    let output = process(input).expect("supported boundary values should be valid");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "65535,1.1111,0.0000,1.1111,false\n",
        )
    );
}

#[test]
fn a_fraction_may_omit_the_leading_zero() {
    let input = concat!("type,client,tx,amount\n", "deposit,1,1,.1250\n",);

    let output = process(input).expect("the decimal fraction should be valid");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "1,0.1250,0.0000,0.1250,false\n",
        )
    );
}

#[test]
fn large_values_beyond_binary_float_precision_round_trip_exactly() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,1,1,9007199254740992.0001\n",
    );

    let output = process(input).expect("the exact decimal should be processed");

    assert_eq!(
        output,
        concat!(
            "client,available,held,total,locked\n",
            "1,9007199254740992.0001,0.0000,9007199254740992.0001,false\n",
        )
    );
}

#[test]
fn malformed_input_fails_before_any_account_output_is_written() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,1,800,2.0000\n",
        "transfer,1,801,1.0000\n",
    );
    let mut output = Vec::new();

    let error = process_csv(input.as_bytes(), &mut output)
        .expect_err("unknown transaction type should fail");

    assert!(matches!(error, ProcessError::InvalidRow { row: 3, .. }));
    assert!(output.is_empty());
}

#[test]
fn control_rows_reject_an_amount_instead_of_silently_ignoring_it() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,1,810,2.0000\n",
        "dispute,1,810,2.0000\n",
    );

    let error = process(input).expect_err("control amount should be rejected");

    assert!(matches!(error, ProcessError::InvalidRow { row: 3, .. }));
}
