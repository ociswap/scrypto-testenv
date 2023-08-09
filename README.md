# scrypto-testenv - Human Scrypto Testing

`scrypto-testenv` is a test environment helper for Radix Scrypto making it easier to write automatic tests. Especially if you need to test many similar test cases.

## Why

Currently writing automatic tests for your Scrypto blueprint requires you to write a lot of boilerplate code, because you can not call your blueprint functions and component methods directly but only via contructing a transaction manifest and execute it afterwards.

This really bloats up your testing code by a lot if done in a naive way - e.g. by just copy-pasting / generating all that boilerplate code.
The issue is that this makes it hard to reason about your test cases and therefore is a potential security risk.

Tests need to be as short and expressive as possible. Especially if you are having many similar cases to test which typically happens if you are having multiple input variables which result in many combinations (of course you can not write tests for every combinations, but you try to identify unique test scenarios - edge cases basically).

We faced these problems during the development of our own Ociswap smart contracts and took our time to think about how to make testing more human and feasible. After some exploration we came up with our own test helper environment `scrypto-testenv`.

The core idea is that it does not replace or fully wrap the current testing tooling but rather provide convenient common helpers which can be used in a cooperative manner. `scrypto-testenv` provides a test schema template which you can use to encapsulate all transaction manifest boilerplate code at one place - freeing your mind to focus on your test case scenarios afterwards.

We open source it now, because we see a pressing need for guidance to improve testing in the Scrypto community.

## Features
- Automatic `TestRunner` and ledger handling
- Provides standard resources like an account and multiple fungible/non-fungible tokens which are just available to be used
- Helper to easily execute transaction manifest (supporting `success`, `failure`, `rejection`)
- Extended `outputs` functionality to also support buckets with `output_buckets`
- Support for instruction labels (inside your custom helper) to make checking `outputs` less error prone and allow for flexible combination and out of order checking
- `nft_ids!` macro, making it easier to assert for NFT ids


## Usage
As an example we have implemented the blueprint `HelloSwap` demonstrating how you could implement your own boilerplate code to make writing automatic tests feasible again.

To start you need to implement your custom boilerplate code (but only once!) using common functionality from `scrypto-testenv`.
You can see the example for `HelloSwap` here [Helper](examples/hello_swap/tests/helper.rs).

Afterwards you are ready to go to use these abstractions and write your tests (automatic type hints are added in the snippet for clearer understanding):
```rust
#[test]
fn test_swap_with_remainder() {
    swap_expect_success(
        y_vault_amount: DEC_10,
        price: dec!(1),
        x_input: dec!(2),
        y_output_expected: y_dec!(1),
        x_remainder_expected: dec!(1)
    )
}
```
Instead of having 30-50 lines of code for one test case using the `TestRunner` directly without abstractions, you are now down to basically 1 single line of code in many cases.

This hugely improves the testing experience and potentially increases smart contract security, because it drastically reduces your mental load while writing your tests.
Allowing you to focus on the things which matter - your test case parameters and expected outputs.

For more examples see the test files:
- [Test Instantiate](examples/hello_swap/tests/test_instantiate.rs)
- [Test Swap](examples/hello_swap/tests/test_swap.rs)

## Contribute
We are looking forward to your feedback and contributions. Additionally, this is work in progress and not fully polished in general and things might change over time.
Besides that, better upstream tooling may make `scrypto-testenv` obsolete.