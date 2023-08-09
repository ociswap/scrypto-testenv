use scrypto::prelude::*;

// MAX_SUPPLY = 1461501637330902918203684832716.283019655932542976
pub const MAX_SUPPLY: Decimal = Decimal(BnumI256::from_digits([0, 0, 4294967296, 0]));
