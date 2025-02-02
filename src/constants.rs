use scrypto::prelude::*;

// MAX_SUPPLY = 5708990770823839524233143877.797980545530986496
pub const MAX_SUPPLY: Decimal = Decimal::from_attos(I192::from_digits([0, 0, 16777216]));

#[test]
fn test_max_supply() {
    assert_eq!(MAX_SUPPLY, Decimal::from_attos(I192::from(2).pow(152)))
}
