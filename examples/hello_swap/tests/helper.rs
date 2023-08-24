use lazy_static::lazy_static;
use radix_engine::{
    blueprints::package::PackageDefinition,
    system::system_modules::execution_trace::ResourceSpecifier::Amount,
};
use scrypto::prelude::*;
use scrypto_testenv::*;
use scrypto_unit::TestRunner;
use std::mem;
use transaction::builder::ManifestBuilder;

lazy_static! {
    static ref PACKAGE: (Vec<u8>, PackageDefinition) = TestRunner::builder()
        .without_trace()
        .build()
        .compile(this_package!());
}

impl TestHelperExecution for HelloSwapTestHelper {
    fn environment(&mut self) -> &mut TestEnvironment {
        &mut self.env
    }
}

pub struct HelloSwapTestHelper {
    env: TestEnvironment,
    pool_address: Option<ComponentAddress>,
    pub price: Option<Decimal>,
}

impl HelloSwapTestHelper {
    pub fn new() -> HelloSwapTestHelper {
        let environment = TestEnvironment::new(&PACKAGE);

        HelloSwapTestHelper {
            env: environment,
            pool_address: None,
            price: None,
        }
    }

    pub fn instantiate(
        &mut self,
        x_address: ResourceAddress,
        y_address: ResourceAddress,
        y_amount: Decimal,
        price: Decimal,
    ) -> &mut HelloSwapTestHelper {
        // with the next ManifestBuilder update this can be simplified to
        // let manifest_builder = mem::take(&mut self.environment.manifest_builder);
        let manifest_builder = mem::replace(&mut self.env.manifest_builder, ManifestBuilder::new());
        self.env.manifest_builder = manifest_builder
            .withdraw_from_account(self.env.account, y_address, y_amount)
            .take_from_worktop(y_address, y_amount, self.name("y_bucket"))
            .with_name_lookup(|builder, lookup| {
                let y_bucket = lookup.bucket(self.name("y_bucket"));
                builder.call_function(
                    self.env.package_address,
                    "HelloSwap",
                    "instantiate",
                    manifest_args!(x_address, y_bucket, price),
                )
            });
        // To support instruction labels we are tracking:
        // instruction_count = the total amount of new instructions added in this function
        // label_instruction_id = (local) instruction id which you want to assign to the label
        // after the ManifestBuilder supports labels upstream this can be simplified
        self.env.new_instruction("instantiate", 3, 2);
        self
    }

    pub fn swap(
        &mut self,
        x_address: ResourceAddress,
        x_amount: Decimal,
    ) -> &mut HelloSwapTestHelper {
        let manifest_builder = mem::replace(&mut self.env.manifest_builder, ManifestBuilder::new());
        self.env.manifest_builder = manifest_builder
            .withdraw_from_account(self.env.account, x_address, x_amount)
            .take_from_worktop(x_address, x_amount, self.name("x_bucket"))
            .with_name_lookup(|builder, lookup| {
                let x_bucket = lookup.bucket(self.name("x_bucket"));
                builder.call_method(self.pool_address.unwrap(), "swap", manifest_args!(x_bucket))
            });
        self.env.new_instruction("swap", 3, 2);
        self
    }

    pub fn instantiate_default(
        &mut self,
        y_amount: Decimal,
        price: Decimal,
        verbose: bool,
    ) -> Receipt {
        self.instantiate(self.x_address(), self.y_address(), y_amount, price);
        let receipt = self.execute_expect_success(verbose);
        let (pool_address, price): (ComponentAddress, Decimal) = receipt.outputs("instantiate")[0];
        self.pool_address = Some(pool_address);
        self.price = Some(price);
        receipt
    }

    pub fn swap_expect_failure(&mut self, x_amount: Decimal) {
        self.swap(self.x_address(), x_amount)
            .execute_expect_failure(true);
    }

    pub fn swap_expect_success(
        &mut self,
        x_amount: Decimal,
        y_amount_expected: Decimal,
        x_remainder_expected: Decimal,
    ) {
        let receipt = self
            .swap(self.x_address(), x_amount)
            .execute_expect_success(true);
        let output_buckets = receipt.output_buckets("swap");

        assert_eq!(
            output_buckets,
            vec![vec![
                Amount(self.y_address(), y_amount_expected),
                Amount(self.x_address(), x_remainder_expected)
            ]],
        );
    }

    pub fn a_address(&self) -> ResourceAddress {
        self.env.a_address
    }

    pub fn b_address(&self) -> ResourceAddress {
        self.env.b_address
    }

    pub fn x_address(&self) -> ResourceAddress {
        self.env.x_address
    }

    pub fn y_address(&self) -> ResourceAddress {
        self.env.y_address
    }

    pub fn v_address(&self) -> ResourceAddress {
        self.env.v_address
    }

    pub fn u_address(&self) -> ResourceAddress {
        self.env.u_address
    }

    pub fn j_nft_address(&self) -> ResourceAddress {
        self.env.j_nft_address
    }

    pub fn k_nft_address(&self) -> ResourceAddress {
        self.env.k_nft_address
    }
}

pub fn instantiate_expect_success(y_amount: Decimal, price: Decimal) {
    let mut helper = HelloSwapTestHelper::new();
    helper.instantiate_default(y_amount, price, true);
}

pub fn instantiate_expect_failure(y_amount: Decimal, price: Decimal) {
    let mut helper = HelloSwapTestHelper::new();
    helper
        .instantiate(helper.x_address(), helper.y_address(), y_amount, price)
        .execute_expect_failure(true);
}

pub fn swap_expect_success(
    y_vault_amount: Decimal,
    price: Decimal,
    x_input: Decimal,
    y_output_expected: Decimal,
    x_remainder_expected: Decimal,
) {
    let mut helper = HelloSwapTestHelper::new();
    helper.instantiate_default(y_vault_amount, price, true);
    helper.swap_expect_success(x_input, y_output_expected, x_remainder_expected);
}

pub fn swap_expect_failure(y_vault_amount: Decimal, price: Decimal, x_input: Decimal) {
    let mut helper = HelloSwapTestHelper::new();
    helper.instantiate_default(y_vault_amount, price, true);
    helper.swap_expect_failure(x_input);
}
