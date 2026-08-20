use super::*;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str_exact(value).expect("test decimal should be exact")
}

fn money(value: &str) -> Money {
    Money::from_transaction_amount(1, decimal(value)).expect("test amount should be valid")
}

#[test]
fn values_with_up_to_four_decimal_places_are_canonicalized_exactly() {
    for (value, canonical) in [
        ("1", "1.0000"),
        ("1.2", "1.2000"),
        ("1.23", "1.2300"),
        ("1.234", "1.2340"),
        ("1.2345", "1.2345"),
    ] {
        assert_eq!(money(value).to_decimal(1), Ok(decimal(canonical)));
    }
}

#[test]
fn transaction_amounts_must_be_positive_and_have_at_most_four_places() {
    assert_eq!(
        Money::from_transaction_amount(1, Decimal::ZERO),
        Err(EngineError::InvalidAmount {
            amount: Decimal::ZERO,
        })
    );
    assert_eq!(
        Money::from_transaction_amount(1, decimal("-1.0000")),
        Err(EngineError::InvalidAmount {
            amount: decimal("-1.0000"),
        })
    );
    assert_eq!(
        Money::from_transaction_amount(1, decimal("0.00001")),
        Err(EngineError::ExcessPrecision {
            amount: decimal("0.00001"),
            scale: 5,
        })
    );
}

#[test]
fn arithmetic_rejects_scale_reduction_instead_of_rounding() {
    let largest = money("792281625142643375935439503.5");
    let smallest = money("0.0001");

    assert_eq!(
        largest.checked_add(smallest, 7),
        Err(EngineError::ArithmeticOverflow { client: 7 })
    );
}

#[test]
fn large_values_keep_their_exact_value_at_a_lower_decimal_scale() {
    let amount = decimal("792281625142643375935439503.5");

    assert_eq!(
        Money::from_transaction_amount(4, amount).and_then(|money| money.to_decimal(4)),
        Ok(amount)
    );
}

#[test]
fn large_arithmetic_succeeds_when_the_exact_result_is_representable() {
    let large = money("792281625142643375935439503.5");
    let half = money("0.5");

    assert_eq!(
        large
            .checked_add(half, 4)
            .and_then(|money| money.to_decimal(4)),
        Ok(decimal("792281625142643375935439504.0"))
    );
}

#[test]
fn arithmetic_past_the_largest_decimal_is_rejected() {
    let largest = money("79228162514264337593543950335");
    let one = money("1");

    assert_eq!(
        largest.checked_add(one, 9),
        Err(EngineError::ArithmeticOverflow { client: 9 })
    );
}
