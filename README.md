# Scrypto project

Basic Decentralized Exchange (DEX) created using Scrypto v1.0 - Radix DLT
Based on Scrypto101: https://academy.radixdlt.com/course/scrypto101

## To run this project

In the project directory, you can run:

- Reset simulator
``` bash 
resim reset 
```

- Create new account as default
``` bash
resim new-account
```

- Create new token with initial supply
```bash
resim new-token-fixed --name Bitcoin --symbol BTC 21000000
```

- Publish the blueprints in the local directory
``` bash
resim publish .
```

Save the address of your new account component to an environment variable to make your life easier later. For example:

``` bash
export account_add=account_sim1c956qr3kxlgypxwst89j9yf24tjc7zxd4up38x37zr6q4jxdx9rhma
export xrd_add=resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3
export btc_add=resource_sim1t4kwg8fa7ldhwh8exe5w4acjhp9v982svmxp3yqa8ncruad4pf6m22
export package_add=package_sim1ph6xspj0xlmspjju2asxg7xnucy7tk387fufs4jrfwsvt85wvqf70a
```

- Call function
``` bash
resim call-function <PACKAGE_ADDRESS> <BLUEPRINT_NAME> <FUNCTION_NAME>
```

Save component address to another environment variable. For example:
``` bash
export component_add=component_sim1cr4tavjnaanmyj9t658rvzrslrlfwhuc96fzj4mnj2c8xnuzenqnzf
```

- Instatiate the blueprint

``` bash
resim run manifests/env/instantiate.rtm
```

- Add liquidity

``` bash
resim run manifests/env/add_liquidity.rtm
```

- remove liquidity

``` bash
resim run manifests/env/remove_liquidity.rtm
```

- swap

``` bash
resim run manifests/env/swap.rtm
```


## Useful links about Radix and Scrypto

- https://academy.radixdlt.com/course/scrypto101

- https://docs.radixdlt.com/docs

- https://docs-babylon.radixdlt.com/main/index.html

## License

[MIT](https://choosealicense.com/licenses/mit/)
