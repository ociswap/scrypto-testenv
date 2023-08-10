use scrypto::prelude::*;
use scrypto_unit::*;
use transaction::{builder::ManifestBuilder, prelude::PreviewFlags};

#[test]
fn test_hello() {
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (public_key, _private_key, account) = test_runner.new_allocated_account();
    let package_address = test_runner.compile_and_publish(this_package!());

    let manifest = ManifestBuilder::new()
        .lock_standard_test_fee(account)
        .call_function(
            package_address,
            "Hello",
            "instantiate_hello",
            manifest_args!(),
        )
        .deposit_batch(account)
        .build();

    let preview_receipt = test_runner.preview_manifest(
        manifest.clone(),
        vec![public_key.clone().into()],
        0,
        PreviewFlags::default(),
    );
    let execution_receipt = test_runner.execute_manifest(
        manifest.clone(),
        vec![NonFungibleGlobalId::from_public_key(&public_key)],
    );
    assert_eq!(
        preview_receipt.expect_commit(true).new_resource_addresses(),
        execution_receipt
            .expect_commit(true)
            .new_resource_addresses()
    );
}
