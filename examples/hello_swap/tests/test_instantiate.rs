mod helper;
use helper::*;
use scrypto::prelude::*;

// The following tests serve as examples and are not comprehensive by any means

const DEC_10: Decimal = dec!(10);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instantiate_price_one() {
        instantiate_expect_success(DEC_10, dec!(1))
    }

    #[test]
    fn test_instantiate_price_zero() {
        instantiate_expect_failure(DEC_10, dec!(0));
    }

    #[test]
    fn test_instantiate_price_negative() {
        instantiate_expect_failure(DEC_10, dec!(-1))
    }
}
