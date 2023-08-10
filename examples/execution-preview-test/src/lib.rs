use scrypto::prelude::*;

#[blueprint]
mod hello {
    struct Hello {}

    impl Hello {
        pub fn instantiate_hello() -> (Global<Hello>, Bucket) {
            let my_bucket: Bucket =
                ResourceBuilder::new_fungible(OwnerRole::None).mint_initial_supply(1000);

            let component = Self {}
                .instantiate()
                .prepare_to_globalize(OwnerRole::None)
                .globalize();

            (component, my_bucket)
        }
    }
}
