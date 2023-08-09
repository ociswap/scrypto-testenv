use scrypto::prelude::*;

#[blueprint]
mod hello_swap {
    struct HelloSwap {
        x_vault: Vault,
        y_vault: Vault,
        price: Decimal,
    }

    impl HelloSwap {
        pub fn instantiate(
            x_address: ResourceAddress,
            y_bucket: Bucket,
            price: Decimal,
        ) -> (Global<HelloSwap>, Decimal) {
            assert!(price > Decimal::ZERO, "Price needs to be positive.");

            let component = Self {
                x_vault: Vault::new(x_address),
                y_vault: Vault::with_bucket(y_bucket),
                price,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize();

            (component, price)
        }

        pub fn swap(&mut self, mut x_bucket: Bucket) -> (Bucket, Bucket) {
            let input = x_bucket.take(self.price);
            let output = self.y_vault.take(1);
            self.x_vault.put(input);
            (output, x_bucket)
        }
    }
}
