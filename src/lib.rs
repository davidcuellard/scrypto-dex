use scrypto::prelude::*;

#[blueprint]
mod scryptodex_module {

    struct ScryptoDex {
        vault_a: FungibleVault,
        vault_b: FungibleVault,
        pool_units_resource_manager: ResourceManager,
        fee: Decimal,
    }

    impl ScryptoDex {

        pub fn instantiate_scryptodex(
            bucket_a: FungibleBucket,
            bucket_b: FungibleBucket,
            fee: Decimal,
        ) -> (Global<ScryptoDex>, FungibleBucket){

            assert!(!bucket_a.is_empty() && !bucket_b.is_empty(), "You must pass in an initial supply of each token");

            assert!(fee >= dec!(0) && fee <= dec!(1), "Invalid fee in thousandths");

            let (address_reservation, component_address) = 
                Runtime::allocate_component_address(ScryptoDex::blueprint_id());

            let pool_units = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(
                    init {
                        "name" => "Pool Units", locked;
                        "symbol" => "PU", locked;
                    }
                ))
                .mint_roles(mint_roles!(
                    minter => rule!(require(global_caller(component_address)));
                    minter_updater => rule!(deny_all);
                ))
                .burn_roles(burn_roles!(
                    burner => rule!(require(global_caller(component_address)));
                    burner_updater => rule!(deny_all);
                ))
                .mint_initial_supply(100);

            let scryptodex = Self{
                vault_a: FungibleVault::with_bucket(bucket_a),
                vault_b: FungibleVault::with_bucket(bucket_b),
                pool_units_resource_manager: pool_units.resource_manager(),
                fee: fee,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .with_address(address_reservation)
            .globalize();

            (scryptodex, pool_units)
        }

        pub fn swap(&mut self, input_tokens: FungibleBucket) -> FungibleBucket{
            // Getting the vault corresponding to the input tokens and the vault 
            // corresponding to the output tokens based on what the input is.
            let (input_tokens_vault, output_tokens_vault): (&mut FungibleVault, &mut FungibleVault) =
            if input_tokens.resource_address() == 
            self.vault_a.resource_address() {
                (&mut self.vault_a, &mut self.vault_b)
            } else if input_tokens.resource_address() == 
            self.vault_b.resource_address() {
                (&mut self.vault_b, &mut self.vault_a)
            } else {
                panic!(
                "The given input tokens do not belong to this liquidity pool"
                )
            };

            // Calculate the output amount of tokens based on the input amount 
            // and the pool fees
            let output_amount: Decimal = (output_tokens_vault.amount()
            * (dec!("1") - self.fee)
            * input_tokens.amount())
            / (input_tokens_vault.amount() + input_tokens.amount() 
            * (dec!("1") - self.fee));

            // Perform the swapping operation
            input_tokens_vault.put(input_tokens);
            output_tokens_vault.take(output_amount)
        }

        pub fn add_liquidity(
            &mut self,
            bucket_a: FungibleBucket,
            bucket_b: FungibleBucket,
        ) -> (FungibleBucket, FungibleBucket, FungibleBucket) {
            // Give the buckets the same names as the vaults
            let (mut bucket_a, mut bucket_b): (FungibleBucket, FungibleBucket) = 
            if bucket_a.resource_address()
                == self.vault_a.resource_address()
                && bucket_b.resource_address() == self.vault_b.resource_address()
            {
                (bucket_a, bucket_b)
            } else if bucket_a.resource_address() == self.vault_b.resource_address()
                && bucket_b.resource_address() == self.vault_a.resource_address()
            {
                (bucket_b, bucket_a)
            } else {
                panic!("One of the tokens does not belong to the pool!")
            };
        
            // Getting the values of `dm` and `dn` based on the sorted buckets
            let dm: Decimal = bucket_a.amount();
            let dn: Decimal = bucket_b.amount();
        
            // Getting the values of m and n from the liquidity pool vaults
            let m: Decimal = self.vault_a.amount();
            let n: Decimal = self.vault_b.amount();
        
            // Calculate the amount of tokens which will be added to each one of 
            //the vaults
            let (amount_a, amount_b): (Decimal, Decimal) =
                if ((m == Decimal::zero()) | (n == Decimal::zero())) 
                    | ((m / n) == (dm / dn)) 
                {
                    // Case 1
                    (dm, dn)
                } else if (m / n) < (dm / dn) {
                    // Case 2
                    (dn * m / n, dn)
                } else {
                    // Case 3
                    (dm, dm * n / m)
                };
        
            // Depositing the amount of tokens calculated into the liquidity pool
            self.vault_a.put(bucket_a.take(amount_a));
            self.vault_b.put(bucket_b.take(amount_b));
        
            // Mint pool units tokens to the liquidity provider
            let pool_units_amount: Decimal =
                if self.pool_units_resource_manager.total_supply().unwrap() == Decimal::zero() {
                    dec!("100.00")
                } else {
                    amount_a * self.pool_units_resource_manager.total_supply().unwrap() / m
                };
            let pool_units: FungibleBucket = self.pool_units_resource_manager.mint(pool_units_amount).as_fungible();
        
            // Return the remaining tokens to the caller as well as the pool units 
            // tokens
            (bucket_a, bucket_b, pool_units)
        }

        pub fn remove_liquidity(&mut self, pool_units: FungibleBucket) -> (FungibleBucket, FungibleBucket) {
    
            assert!(
                pool_units.resource_address() == self.pool_units_resource_manager.address(),
                "Wrong token type passed in"
            );
        
            // Calculate the share based on the input LP tokens.
            let share = pool_units.amount() / 
                self.pool_units_resource_manager.total_supply().unwrap();
        
            // Burn the LP tokens received
            pool_units.burn();
        
            // Return the withdrawn tokens
            (
                self.vault_a.take(self.vault_a.amount() * share),
                self.vault_b.take(self.vault_b.amount() * share),
            )
        }

    }
    
}
