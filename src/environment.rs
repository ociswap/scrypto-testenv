use radix_engine::{
    system::system_modules::execution_trace::{ResourceSpecifier, WorktopChange},
    transaction::TransactionReceipt,
};
use scrypto::prelude::*;
use scrypto_unit::TestRunner;
use std::{mem, path::Path};
use transaction::{builder::ManifestBuilder, prelude::*};

use crate::MAX_SUPPLY;

#[macro_export]
macro_rules! nft_ids {
    ($($x:expr),*) => {
        {
            let mut temp_set = BTreeSet::new();
            $(
                temp_set.insert(NonFungibleLocalId::Integer($x.into()));
            )*
            temp_set
        }
    };
}

const INSTRUCTION_COUNTER_INIT: usize = 1; // lock_standard_test_fee will be added always as first instruction automatically

pub enum TestAddress {
    A,
    B,
    X,
    Y,
    U,
    V,
}

pub struct TestEnvironment {
    pub test_runner: TestRunner,
    pub manifest_builder: ManifestBuilder,

    pub package_address: PackageAddress,
    pub public_key: Secp256k1PublicKey,
    pub account: ComponentAddress,

    pub admin_badge_address: ResourceAddress,
    pub a_address: ResourceAddress,
    pub b_address: ResourceAddress,
    pub x_address: ResourceAddress,
    pub y_address: ResourceAddress,
    pub u_address: ResourceAddress,
    pub v_address: ResourceAddress,
    pub j_nft_address: ResourceAddress,
    pub k_nft_address: ResourceAddress,

    pub instruction_counter: usize,
    instruction_ids_by_label: HashMap<String, Vec<usize>>,
}

impl TestEnvironment {
    pub fn new<P>(package_dir: P) -> Self
    where
        P: AsRef<Path>,
    {
        let mut test_runner = TestRunner::builder().without_trace().build();

        let (public_key, _private_key, account) = test_runner.new_allocated_account();
        let package_address = test_runner.compile_and_publish(package_dir);
        let manifest_builder = ManifestBuilder::new().lock_standard_test_fee(account);

        let admin_badge_address =
            test_runner.create_fungible_resource(dec!(1), DIVISIBILITY_NONE, account);
        let a_address = test_runner.create_fungible_resource(
            MAX_SUPPLY,
            DIVISIBILITY_MAXIMUM,
            account,
        );
        let b_address = test_runner.create_fungible_resource(
            MAX_SUPPLY,
            DIVISIBILITY_MAXIMUM,
            account,
        );
        let (x_address, y_address) = sort_addresses(a_address, b_address);

        let u_address = test_runner.create_fungible_resource(
            dec!(1000000000),
            DIVISIBILITY_MAXIMUM,
            account,
        );
        let v_address = test_runner.create_fungible_resource(
            dec!(10000000),
            DIVISIBILITY_MAXIMUM,
            account,
        );
        let j_nft_address = test_runner.create_non_fungible_resource(account);
        let k_nft_address = test_runner.create_non_fungible_resource(account);

        Self {
            test_runner,
            manifest_builder,
            package_address,
            public_key,
            account,

            admin_badge_address,
            a_address,
            b_address,
            x_address,
            y_address,
            u_address,
            v_address,
            j_nft_address,
            k_nft_address,

            instruction_counter: INSTRUCTION_COUNTER_INIT,
            instruction_ids_by_label: HashMap::new(),
        }
    }

    pub fn new_instruction(
        &mut self,
        label: &str,
        instruction_count: usize,
        label_instruction_id: usize,
    ) {
        self.instruction_ids_by_label
            .entry(label.to_string())
            .or_default()
            .push(self.instruction_counter + label_instruction_id);
        self.instruction_counter += instruction_count;
    }
}

pub trait TestHelperExecution {
    fn environment(&mut self) -> &mut TestEnvironment;

    fn execute(&mut self, verbose: bool) -> Receipt {
        let account_component = self.environment().account;
        let public_key = self.environment().public_key;
        let manifest_builder = mem::replace(
            &mut self.environment().manifest_builder,
            ManifestBuilder::new(),
        );
        let manifest = manifest_builder.deposit_batch(account_component).build();
        let preview_receipt = self.environment().test_runner.preview_manifest(
            manifest.clone(),
            vec![public_key.clone().into()],
            0,
            PreviewFlags::default(),
        );
        let execution_receipt = self.environment().test_runner.execute_manifest(
            manifest.clone(),
            vec![NonFungibleGlobalId::from_public_key(&public_key)],
        );
        if verbose {
            println!("{:?}", execution_receipt);
        }
        let instruction_mapping = self.environment().instruction_ids_by_label.clone();
        self.reset_instructions();
        let manifest_builder = mem::replace(
            &mut self.environment().manifest_builder,
            ManifestBuilder::new(),
        );
        self.environment().manifest_builder =
            manifest_builder.lock_standard_test_fee(self.environment().account);
        Receipt {
            execution_receipt,
            preview_receipt,
            instruction_ids_by_label: instruction_mapping,
        }
    }

    fn execute_expect_success(&mut self, verbose: bool) -> Receipt {
        let receipt = self.execute(verbose);
        receipt.execution_receipt.expect_commit_success();
        receipt
    }

    fn execute_expect_failure(&mut self, verbose: bool) -> Receipt {
        let receipt = self.execute(verbose);
        receipt.execution_receipt.expect_commit_failure();
        receipt
    }

    fn execute_expect_rejection(&mut self, verbose: bool) -> Receipt {
        let receipt = self.execute(verbose);
        receipt.execution_receipt.expect_rejection();
        receipt
    }

    fn name(&mut self, name: &str) -> String {
        format!("{}_{}", name, self.environment().instruction_counter)
    }

    fn reset_instructions(&mut self) {
        self.environment().instruction_ids_by_label = HashMap::new();
        self.environment().instruction_counter = INSTRUCTION_COUNTER_INIT;
    }
}

pub struct Receipt {
    pub execution_receipt: TransactionReceipt,
    pub preview_receipt: TransactionReceipt,
    pub instruction_ids_by_label: HashMap<String, Vec<usize>>,
}

impl Receipt {
    pub fn output_buckets(&self, instruction_label: &str) -> Vec<Vec<ResourceSpecifier>> {
        self.preview_receipt
            .output_buckets(self.instruction_ids(instruction_label))
    }

    pub fn outputs<T>(&self, instruction_label: &str) -> Vec<T>
    where
        T: ScryptoDecode,
    {
        self.execution_receipt
            .outputs(self.instruction_ids(instruction_label))
    }

    fn instruction_ids(&self, instruction_label: &str) -> Vec<usize> {
        self.instruction_ids_by_label
            .get(&instruction_label.to_string())
            .unwrap()
            .clone()
    }
}

pub trait TransactionReceiptOutputBuckets {
    fn output_buckets(&self, instruction_ids: Vec<usize>) -> Vec<Vec<ResourceSpecifier>>;
    fn outputs<T>(&self, instruction_ids: Vec<usize>) -> Vec<T>
    where
        T: ScryptoDecode;
}

impl TransactionReceiptOutputBuckets for TransactionReceipt {
    fn output_buckets(&self, instruction_ids: Vec<usize>) -> Vec<Vec<ResourceSpecifier>> {
        let worktop_changes = self
            .expect_commit_success()
            .execution_trace
            .worktop_changes();
        instruction_ids
            .iter()
            .filter_map(|id| {
                let instruction_worktop_changes = worktop_changes.get(id).unwrap();
                Some(
                    instruction_worktop_changes
                        .iter()
                        .filter_map(|change| match change {
                            WorktopChange::Put(resource_specifier) => {
                                Some(resource_specifier.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                )
            })
            .collect()
    }

    fn outputs<T>(&self, instruction_ids: Vec<usize>) -> Vec<T>
    where
        T: ScryptoDecode,
    {
        instruction_ids
            .iter()
            .filter_map(|id| Some(self.expect_commit_success().output(*id)))
            .collect()
    }
}

pub fn sort_addresses(
    a_address: ResourceAddress,
    b_address: ResourceAddress,
) -> (ResourceAddress, ResourceAddress) {
    if a_address < b_address {
        (a_address, b_address)
    } else {
        (b_address, a_address)
    }
}

#[test]
fn test_nft_ids() {
    assert_eq!(
        nft_ids!(1, 3, 2),
        BTreeSet::from([
            NonFungibleLocalId::Integer((1).into()),
            NonFungibleLocalId::Integer((2).into()),
            NonFungibleLocalId::Integer((3).into()),
        ])
    )
}
