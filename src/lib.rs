use scrypto::prelude::*;

#[blueprint]
mod scryptodex_module {

    //Create access rules
    enable_method_auth! {
        roles {
            admin => updatable_by: [OWNER];
         },
        methods {
            swap => PUBLIC;
            add_liquidity => PUBLIC;
            remove_liquidity => PUBLIC;
            get_price => restrict_to: [admin, OWNER];
        }
    }

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
        ) -> (Global<ScryptoDex>, FungibleBucket, FungibleBucket){

            assert!(!bucket_a.is_empty() && !bucket_b.is_empty(), "You must pass in an initial supply of each token");

            assert!(fee >= dec!(0) && fee <= dec!(1), "Fee must be between 0 and 1");

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

            // Create Admin badge
            // Owner role updatable by Owner
            let admin_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata!(init{"name"=>"admin badge", locked;}))
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let scryptodex = Self{
                vault_a: FungibleVault::with_bucket(bucket_a),
                vault_b: FungibleVault::with_bucket(bucket_b),
                pool_units_resource_manager: pool_units.resource_manager(),
                fee: fee,
            }
            .instantiate()
            .prepare_to_globalize(
                OwnerRole::Fixed(rule!(require(admin_badge.resource_address())))
            )
            .roles(roles!( 
                admin => rule!(require(admin_badge.resource_address())); 
            ))
            .with_address(address_reservation)
            .globalize();

            (scryptodex, pool_units, admin_badge)
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

        pub fn get_price(&mut self) {      

            // let (mut name_a, mut name_b): (&str, &str) = 
            // if self.vault_a.resource_address() ==  resource_address("resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3")
            //     && self.vault_b.resource_address() == "resource_sim1t4kwg8fa7ldhwh8exe5w4acjhp9v982svmxp3yqa8ncruad4pf6m22"
            // {
            //     ("XRD", "BTC")
            // } else if self.vault_b.resource_address() == "resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3"
            // && self.vault_a.resource_address() == "resource_sim1t4kwg8fa7ldhwh8exe5w4acjhp9v982svmxp3yqa8ncruad4pf6m22"
            // {
            //     ("BTC", "XRD")
            // } else {
            //     panic!("One of the tokens does not belong to the pool!")
            // };        

            
            // sys_bech32_encode_address( "resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3" )

            // ComponentAddress::try_from_bech32(self.vault_a.resource_address(), str);

            // ResourceManager::from_address("resource_sim1nfq8ezgchvcph978k2kn5v74rggeyh9nxqmley9a2pqjygfcua05sg");

            // info!("Resource type: {}", self.vault_a );


            // Get vault's symbol
            let _symbol_a: String = ResourceManager::from_address(self.vault_a.resource_address()).get_metadata::<&str, String>("symbol").unwrap().unwrap().into();
            let _symbol_b: String = ResourceManager::from_address(self.vault_b.resource_address()).get_metadata::<&str, String>("symbol").unwrap().unwrap().into();


            info!(
                "LP {} vault quantity: {} ", 
                _symbol_a,
                self.vault_a.amount()
            );

            info!(
                "LP {} vault quantity: {} ", 
                _symbol_b, 
                self.vault_b.amount()
            );

            info!(
                "Price {}/{}: {} ", 
                _symbol_a, 
                _symbol_b, 
                self.vault_a.amount() / self.vault_b.amount()
            );

            info!(
                "LP total supply: {} ", 
                self.pool_units_resource_manager.total_supply().unwrap()
            );
        }

    }
}
