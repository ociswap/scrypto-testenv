use scrypto::prelude::*;
mod helper;
use helper::*;

// The following tests serve as examples and are not comprehensive by any means

const DEC_10: Decimal = dec!(10);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_swap() {
        swap_expect_success(DEC_10, dec!(1), dec!(1), dec!(1), dec!(0))
    }

    #[test]
    fn test_swap_with_remainder() {
        swap_expect_success(DEC_10, dec!(3), dec!(5), dec!(1), dec!(2))
    }

    #[test]
    fn test_swap_not_enough_input() {
        swap_expect_failure(DEC_10, dec!(2), dec!(1));
    }
}
