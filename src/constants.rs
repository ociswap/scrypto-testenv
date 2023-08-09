use scrypto::prelude::*;

// MAX_SUPPLY = 1000000000000000000
// TODO needs updating, but latest announced MAX_SUPPLY = 2**160 seems to be too large
pub const MAX_SUPPLY: Decimal = Decimal(BnumI256::from_digits([
    12919594847110692864,
    54210108624275221,
    0,
    0,
]));
