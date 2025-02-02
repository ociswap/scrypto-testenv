use radix_engine::{
    blueprints::package::PackageDefinition,
    system::system_modules::execution_trace::{ResourceSpecifier, WorktopChange},
    transaction::TransactionReceipt,
    vm::NoExtension,
};
use radix_substate_store_impls::memory_db::InMemorySubstateDatabase;
use radix_transactions::{builder::ManifestBuilder, prelude::*};
use scrypto::prelude::*;
use scrypto_test::ledger_simulator::{
    LedgerSimulator, LedgerSimulatorBuilder, LedgerSimulatorSnapshot,
};
use std::hash::Hash;
use std::{
    mem,
    path::{Path, PathBuf},
};

use crate::MAX_SUPPLY;

#[macro_export]
macro_rules! nft_id {
    ($x:expr) => {
        NonFungibleLocalId::Integer($x.into())
    };
}

#[macro_export]
macro_rules! nft_ids {
    ($($x:expr),*) => {
        {
            let mut temp_set = IndexSet::new();
            $(
                temp_set.insert(NonFungibleLocalId::Integer($x.into()));
            )*
            temp_set
        }
    };
}

const INSTRUCTION_COUNTER_INIT: usize = 1; // lock_standard_test_fee will be added always as first instruction automatically

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

type CompiledPackage = (Vec<u8>, PackageDefinition);

lazy_static! {
    static ref TEST_ENVIRONMENT_CACHE: RwLock<HashMap<BTreeSet<PathBuf>, TestEnvironmentSnapshot>> =
        RwLock::new(HashMap::new());
    static ref PACKAGE_CACHE: RwLock<HashMap<PathBuf, CompiledPackage>> =
        RwLock::new(HashMap::new());
}

fn get_cache<K: Hash + Eq, V: Clone>(cache: &RwLock<HashMap<K, V>>, key: &K) -> Option<V> {
    let read_lock = cache.read().unwrap();
    match read_lock.get(key) {
        Some(state) => Some(state.clone()),
        None => None,
    }
}

// Optimized getter for TEST_ENVIRONMENT_CACHE, avoids unnecessary clone with direct revive
fn get_cache_test_environment(key: &BTreeSet<PathBuf>) -> Option<TestEnvironment> {
    let read_lock = TEST_ENVIRONMENT_CACHE.read().unwrap();
    match read_lock.get(key) {
        Some(snapshot) => Some(snapshot.revive()),
        None => None,
    }
}

fn write_cache<K: Hash + Eq + Clone, V>(cache: &RwLock<HashMap<K, V>>, key: K, value: V) {
    let mut write_lock = cache.write().unwrap();
    write_lock.entry(key).or_insert(value);
}

// OPTIMIZE: can be optimized in the future by checking whether a test_environment is being generated,
// even if it was found not to exist, avoiding concurrent generation of the same environment,
// and then discarding one of the copies after the first one is written.
// Two possible approaches:
// 1) Creating two static RwLock Hashmap variables, one for PACKAGE_CACHE and the other for TEST_ENVIRONMENT_CACHE
//    That would state, for each possible cache entry, whether it is being generated or not
// 2) Instead of the cache variables holding the final object, they would be wrapped in a
//    new CachedObject struct, that has as fields and Option<T> and a bool "generation"
//    that would be set to true when a thread starts to generate that object

pub enum TestAddress {
    A,
    B,
    X,
    Y,
    U,
    V,
}

pub struct TestEnvironment {
    pub test_runner: LedgerSimulator<NoExtension, InMemorySubstateDatabase>,
    pub manifest_builder: ManifestBuilder,

    pub package_addresses: HashMap<String, PackageAddress>,
    pub public_key: Secp256k1PublicKey,
    pub account: ComponentAddress,
    pub dapp_definition: ComponentAddress,

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
    pub fn new<T: AsRef<Path> + Ord>(packages: HashMap<&str, T>) -> Self {
        let packages: HashMap<&str, PathBuf> = packages
            .iter()
            .map(|(&package_name, package_dir)| (package_name, package_dir.as_ref().to_path_buf()))
            .into_iter()
            .collect();

        let package_dirs: BTreeSet<PathBuf> = packages.values().cloned().collect();
        let test_environment_cached = get_cache_test_environment(&package_dirs);

        if let Some(test_environment_) = test_environment_cached {
            return test_environment_;
        }

        let mut test_environment_new =
            get_cache_test_environment(&BTreeSet::new()).unwrap_or_else(|| {
                let test_environment_empty_ = TestEnvironment::generate_new_test_environment();
                write_cache(
                    &TEST_ENVIRONMENT_CACHE,
                    BTreeSet::new(), // Cache empty (packageless) environment
                    test_environment_empty_.create_snapshot(),
                );
                test_environment_empty_
            });

        if packages.is_empty() {
            return test_environment_new;
        }

        // Leaving package publishing for last, means that there will be nothing
        // changing the network state before the account/tokens/etc are created
        // This means we can use a snapshot of the an empty (package-less) TestEnvironment
        // and just publish packages on top of it, with the fields of the TestEnvironment
        // (account/tokens/etc) remaining valid

        test_environment_new.compile_and_publish_packages(packages);
        write_cache(
            &TEST_ENVIRONMENT_CACHE,
            package_dirs, // Cache TestEnvironment with new packages
            test_environment_new.create_snapshot(),
        );
        test_environment_new
    }

    /// Retrieves a TestEnvironment from the snapshot
    /// IMPORTANT: The states of the following fields are not recovered:
    /// - MenifestBuilder
    /// - instruction_counter
    /// - instruction_ids_by_label
    pub fn from_snapshot(snapshot: TestEnvironmentSnapshot) -> Self {
        snapshot.revive()
    }

    fn generate_new_test_environment() -> TestEnvironment {
        let mut test_runner = LedgerSimulatorBuilder::new().without_kernel_trace().build();

        let (public_key, _private_key, account) = test_runner.new_allocated_account();
        let (_, _, dapp_definition) = test_runner.new_allocated_account();

        let manifest_builder = ManifestBuilder::new().lock_standard_test_fee(account);

        let package_addresses: HashMap<String, PackageAddress> = HashMap::new();

        let admin_badge_address =
            test_runner.create_fungible_resource(dec!(1), DIVISIBILITY_NONE, account);
        let a_address = test_runner.create_fungible_resource_advanced(
            MAX_SUPPLY,
            DIVISIBILITY_MAXIMUM,
            account,
            metadata! {
                init {
                    "name" => "Test token A".to_owned(), locked;
                    "symbol" => "A".to_owned(), locked;
                }
            },
        );
        let b_address = test_runner.create_fungible_resource_advanced(
            MAX_SUPPLY,
            DIVISIBILITY_MAXIMUM,
            account,
            metadata! {
                init {
                    "name" => "Test token B".to_owned(), locked;
                    "symbol" => "B".to_owned(), locked;
                }
            },
        );
        let (x_address, y_address) = sort_addresses(a_address, b_address);

        let u_address =
            test_runner.create_fungible_resource(dec!(1000000000), DIVISIBILITY_MAXIMUM, account);
        let v_address =
            test_runner.create_fungible_resource(dec!(10000000), DIVISIBILITY_MAXIMUM, account);
        let j_nft_address = test_runner.create_non_fungible_resource(account);
        let k_nft_address = test_runner.create_non_fungible_resource(account);

        let test_environment = Self {
            test_runner,
            manifest_builder,
            package_addresses,
            public_key,
            account,
            dapp_definition,

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
        };

        test_environment
    }

    /// Compiles and Publishes Packages
    ///
    /// IMPORTANT: Prefer usage of TestEnvironment::new(packages_map) over
    /// TestEnvironment::new(empty_package_map) + test_environment.compile_and_publish_packages,
    /// since the first results in caching of clean environment states + respective packages,
    /// speeding up future calls
    pub fn compile_and_publish_packages(&mut self, packages: HashMap<&str, PathBuf>) {
        let package_addresses: HashMap<String, PackageAddress> = packages
            .into_iter()
            .map(|(package_name, package_dir)| {
                let cache_result: Option<CompiledPackage> = get_cache(&PACKAGE_CACHE, &package_dir);
                let compiled_package = match cache_result {
                    Some(compiled_package) => compiled_package,
                    None => {
                        let compiled_package = self.test_runner.compile(&package_dir);
                        write_cache(&PACKAGE_CACHE, package_dir, compiled_package.clone());
                        compiled_package
                    }
                };
                let package_address = self.test_runner.publish_package(
                    compiled_package,
                    BTreeMap::new(),
                    OwnerRole::Updatable(rule!(require(self.admin_badge_address))),
                );
                (package_name.to_string(), package_address)
            })
            .collect();

        self.package_addresses.extend(package_addresses);
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

    pub fn package_address(&self, package_name: &str) -> PackageAddress {
        *self
            .package_addresses
            .get(package_name)
            .expect(format!("Package {:?} not found", package_name).as_str())
    }

    /// Creates and retrieves snapshot of the TestEnvironment
    /// IMPORTANT: The states of the following fields are dropped:
    /// - MenifestBuilder
    /// - instruction_counter
    /// - instruction_ids_by_label
    pub fn create_snapshot(&self) -> TestEnvironmentSnapshot {
        TestEnvironmentSnapshot::from(self)
    }
}

/// NOTE: This should only be used for single clones,
/// since it clones by taking a snapshot and then recovering from it.
/// For the creation of many clones, it is advised to manually snapshot
/// and then creating as many TestEnvironments as needed from
/// that snapshot
impl Clone for TestEnvironment {
    fn clone(&self) -> Self {
        self.create_snapshot().revive()
    }
}

pub struct TestEnvironmentSnapshot {
    pub test_runner_snapshot: LedgerSimulatorSnapshot,

    pub package_addresses: HashMap<String, PackageAddress>,
    pub public_key: Secp256k1PublicKey,
    pub account: ComponentAddress,
    pub dapp_definition: ComponentAddress,

    pub admin_badge_address: ResourceAddress,
    pub a_address: ResourceAddress,
    pub b_address: ResourceAddress,
    pub x_address: ResourceAddress,
    pub y_address: ResourceAddress,
    pub u_address: ResourceAddress,
    pub v_address: ResourceAddress,
    pub j_nft_address: ResourceAddress,
    pub k_nft_address: ResourceAddress,
}

impl TestEnvironmentSnapshot {
    /// Creates snapshot of the TestEnvironment
    /// IMPORTANT: The states of the following fields are dropped:
    /// - MenifestBuilder
    /// - instruction_counter
    /// - instruction_ids_by_label
    pub fn from(test_environment: &TestEnvironment) -> TestEnvironmentSnapshot {
        Self {
            test_runner_snapshot: test_environment.test_runner.create_snapshot(),
            package_addresses: test_environment.package_addresses.clone(),
            public_key: test_environment.public_key.clone(),
            account: test_environment.account.clone(),
            dapp_definition: test_environment.dapp_definition.clone(),
            admin_badge_address: test_environment.admin_badge_address.clone(),
            a_address: test_environment.a_address.clone(),
            b_address: test_environment.b_address.clone(),
            x_address: test_environment.x_address.clone(),
            y_address: test_environment.y_address.clone(),
            u_address: test_environment.u_address.clone(),
            v_address: test_environment.v_address.clone(),
            j_nft_address: test_environment.j_nft_address.clone(),
            k_nft_address: test_environment.k_nft_address.clone(),
        }
    }

    /// Retrieves a TestEnvironment from the snapshot
    /// IMPORTANT: The states of the following fields are not recovered:
    /// - MenifestBuilder
    /// - instruction_counter
    /// - instruction_ids_by_label
    pub fn revive(&self) -> TestEnvironment {
        TestEnvironment {
            test_runner: LedgerSimulatorBuilder::new()
                .without_kernel_trace()
                .build_from_snapshot(self.test_runner_snapshot.clone()),
            manifest_builder: ManifestBuilder::new().lock_standard_test_fee(self.account),

            package_addresses: self.package_addresses.clone(),
            public_key: self.public_key.clone(),
            account: self.account.clone(),
            dapp_definition: self.dapp_definition.clone(),

            admin_badge_address: self.admin_badge_address.clone(),
            a_address: self.a_address.clone(),
            b_address: self.b_address.clone(),
            x_address: self.x_address.clone(),
            y_address: self.y_address.clone(),
            u_address: self.u_address.clone(),
            v_address: self.v_address.clone(),
            j_nft_address: self.j_nft_address.clone(),
            k_nft_address: self.k_nft_address.clone(),

            instruction_counter: INSTRUCTION_COUNTER_INIT,
            instruction_ids_by_label: HashMap::new(),
        }
    }
}

pub trait TestHelperExecution {
    fn env(&mut self) -> &mut TestEnvironment;

    fn execute(&mut self, verbose: bool) -> Receipt {
        let account_component = self.env().account;
        let public_key = self.env().public_key;
        let manifest_builder =
            mem::replace(&mut self.env().manifest_builder, ManifestBuilder::new());
        let manifest = manifest_builder
            .deposit_entire_worktop(account_component)
            .build();
        let preview_receipt = self.env().test_runner.preview_manifest(
            manifest.clone(),
            vec![public_key.clone().into()],
            0,
            PreviewFlags::default(),
        );
        let execution_receipt = self.env().test_runner.execute_manifest(
            manifest.clone(),
            vec![NonFungibleGlobalId::from_public_key(&public_key)],
        );
        if verbose {
            println!("{:?}", execution_receipt);
        }
        let instruction_mapping = self.env().instruction_ids_by_label.clone();
        self.reset_instructions();
        let manifest_builder =
            mem::replace(&mut self.env().manifest_builder, ManifestBuilder::new());
        self.env().manifest_builder = manifest_builder.lock_standard_test_fee(self.env().account);
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
        format!("{}_{}", name, self.env().instruction_counter)
    }

    fn reset_instructions(&mut self) {
        self.env().instruction_ids_by_label = HashMap::new();
        self.env().instruction_counter = INSTRUCTION_COUNTER_INIT;
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
            .expect(&format!("Can't find instruction '{}'", instruction_label))
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
        match &self.expect_commit_success().execution_trace {
            None => vec![],
            Some(execution_trace) => {
                let worktop_changes = execution_trace.worktop_changes();
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
        }
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

pub trait GetResourceAddress {
    fn address(&self) -> ResourceAddress;
}

impl GetResourceAddress for ResourceSpecifier {
    fn address(&self) -> ResourceAddress {
        match self {
            ResourceSpecifier::Amount(address, _) => *address,
            ResourceSpecifier::Ids(address, _) => *address,
        }
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

pub trait CreateFungibleResourceAdvanced {
    fn create_fungible_resource_advanced(
        &mut self,
        amount: Decimal,
        divisibility: u8,
        account: ComponentAddress,
        metadata: ModuleConfig<MetadataInit>,
    ) -> ResourceAddress;
}

impl CreateFungibleResourceAdvanced for LedgerSimulator<NoExtension, InMemorySubstateDatabase> {
    fn create_fungible_resource_advanced(
        &mut self,
        amount: Decimal,
        divisibility: u8,
        account: ComponentAddress,
        metadata: ModuleConfig<MetadataInit>,
    ) -> ResourceAddress {
        let manifest = ManifestBuilder::new()
            .lock_fee_from_faucet()
            .create_fungible_resource(
                OwnerRole::None,
                true,
                divisibility,
                FungibleResourceRoles::default(),
                metadata,
                Some(amount),
            )
            .try_deposit_entire_worktop_or_abort(account, None)
            .build();
        let receipt = self.execute_manifest(manifest, vec![]);
        receipt.expect_commit(true).new_resource_addresses()[0]
    }
}

#[test]
fn test_nft_id() {
    assert_eq!(nft_id!(3), NonFungibleLocalId::Integer((3).into()))
}

#[test]
fn test_nft_ids() {
    assert_eq!(
        nft_ids!(1, 3, 2),
        IndexSet::from([
            NonFungibleLocalId::Integer((1).into()),
            NonFungibleLocalId::Integer((2).into()),
            NonFungibleLocalId::Integer((3).into()),
        ])
    )
}

#[test]
fn test_test_environment_snapshot() {
    let packages: HashMap<&str, &str> = HashMap::new();
    let test_environment = TestEnvironment::new(packages);
    let test_environment_new = TestEnvironmentSnapshot::from(&test_environment).revive();

    assert!(test_environment.package_addresses == test_environment_new.package_addresses);
    assert!(test_environment.public_key == test_environment_new.public_key);
    assert!(test_environment.account == test_environment_new.account);
    assert!(test_environment.dapp_definition == test_environment_new.dapp_definition);
    assert!(test_environment.admin_badge_address == test_environment_new.admin_badge_address);
    assert!(test_environment.a_address == test_environment_new.a_address);
    assert!(test_environment.b_address == test_environment_new.b_address);
    assert!(test_environment.x_address == test_environment_new.x_address);
    assert!(test_environment.y_address == test_environment_new.y_address);
    assert!(test_environment.u_address == test_environment_new.u_address);
    assert!(test_environment.v_address == test_environment_new.v_address);
    assert!(test_environment.j_nft_address == test_environment_new.j_nft_address);
    assert!(test_environment.k_nft_address == test_environment_new.k_nft_address);
}
