<div align="center">
  <h1>raydium-library</h1>
</div>


## Overview

- **The repo contains two parts: raydium contracts instructions with amounts calculation(raydium-library) and command client(raydium-cli) as the entry point for instruction execution parameters used for client side in rust.**
- **Currently all the input and output tokens must be coverted to WSOL manually if you expect SOL.**


## Environment Setup
1. Rustc version is less than 1.80.0.
2. Solana version is less than 1.18.0.
3. Anchor version is 0.29.

### Note
Because of the stack overflow error while compiling cp swap and clmm contracts depends on anchor 0.30.1, we still use the outdated dependencies.
Which will be optimized in anchor 0.31([PR](https://github.com/coral-xyz/anchor/pull/2939)).
After the release of anchor 0.31, we will update the related dependencies.


## Build
Clone the repository and enter the source code directory.
```bash
git clone https://github.com/raydium-io/raydium-library
cd raydium-library
```
### Mainnet Build
```bash
cargo build --release
```
### Devnet Build
```bash
cargo build --release --features devnet
```
Then copy the raydium file in the target/release directory to wherever you need.


## Configuration
- **The configuration here consists of three parts: the default configuration based on the specified build features, the configuration file override configuration, and the command line override configuration.**
1. The default configuration.

|Config name             |Build default                                |Build with specified devnet features         |
|------------------------|---------------------------------------------|---------------------------------------------|
|http_url                |https://api.mainnet-beta.solana.com          |https://api.mainnet-beta.solana.com          |
|ws_url                  |wss://api.mainnet-beta.solana.com            |wss://api.devnet.solana.com                  |
|wallet_path             |empty                                        |empty                                        |
|raydium_clmm_program    |CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK |devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH  |
|raydium_cp_swap_program |CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C |CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW |
|raydium_amm_program     |675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 |HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8 |
|openbook_program        |srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX  |EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj |
|slippage_bps            |100                                          |100                                          |
|simulate                |false                                        |false                                        |

2. User can override the default configuration with configuration file named Raydium.toml.
This configuration file must be in the same directory as the raydium executable file.
And the format is as follows:
```rust
[cluster]
http_url = "https://api.mainnet-beta.solana.com"
ws_url = "wss://api.mainnet-beta.solana.com"

[program]
raydium_clmm_program = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
raydium_cp_swap_program = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"
raydium_amm_program = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
openbook_program = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"

[info]
slippage_bps = 100
wallet_path = ""
```

3. User can also use the command line to override all the above configurations.
```bash
Usage: raydium [OPTIONS] <COMMAND>

Commands:
  cpswap
  clmm
  amm
  help    Print this message or the help of the given subcommand(s)

Options:
      --config.http <HTTP_URL>
      --config.ws <WS_URL>
      --config.wallet <WALLET_PATH>
      --config.clmm_program <RAYDIUM_CLMM_PROGRAM>
      --config.cp_program <RAYDIUM_CP_SWAP_PROGRAM>
      --config.amm_program <RAYDIUM_AMM_PROGRAM>
      --config.openbook_program <OPENBOOK_PROGRAM>
      --config.slippage <SLIPPAGE_BPS>
  -s, --simulate
  -h, --help 
```


## Customize client
- **You can also customize your own client tools through code.**
1. Add dependencies in your Cargo.toml
```rust
[features]
# default is mainnet
devnet = [
    "amm-cli/devnet",
    "clmm-cli/devnet",
    "cpswap-cli/devnet",
    "common/devnet",
]

[dependencies]
amm-cli = { git = "https://github.com/raydium-io/raydium-library" }
clmm-cli = { git = "https://github.com/raydium-io/raydium-library" }
cpswap-cli = { git = "https://github.com/raydium-io/raydium-library" }
common = { git = "https://github.com/raydium-io/raydium-library" }
spl-token = { version = "4.0.0", features = ["no-entrypoint"] }
spl-associated-token-account = { version = "2.2.0", features = [
    "no-entrypoint",
] }
spl-token-2022 = { version = "0.9.0", features = ["no-entrypoint"] }
solana-client = "<1.17.0"
solana-sdk = "<1.17.0"
anyhow = "1.0.53"
clap = { version = "4.1.8", features = ["derive"] }
```

2. Importing the crates you need.
```rust
use anyhow::{Ok, Result};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer};
use std::sync::Arc;

use {
    amm_cli::{self, AmmCommands},
    clmm_cli::{self, ClmmCommands},
    common::{common_types, common_utils, rpc},
    cpswap_cli::{self, CpSwapCommands},
};
```

3. Custom configuration parameters in your code.
```rust
// default config
let mut config = common_types::CommonConfig::default();
// Replace the default configuration parameters you need
config.set_cluster("http", "ws");
config.set_wallet("your wallet path");
config.set_amm_program("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
config.set_openbook_program("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX");
config.set_slippage(50);
```

4. Constructing a signed storage object.
```rust
let payer = common_utils::read_keypair_file(&config.wallet())?;
let fee_payer = payer.pubkey();
let mut signing_keypairs: Vec<Arc<dyn Signer>> = Vec::new();
let payer: Arc<dyn Signer> = Arc::new(payer);
if !signing_keypairs.contains(&payer) {
    signing_keypairs.push(payer);
}
```

5. Constructing instruction you need.
```rust
let subcmd = AmmCommands::CreatePool {
    market: Pubkey::from_str("openbook market address").unwrap(),
    coin_mint: Pubkey::from_str("coin mint address").unwrap(),
    pc_mint: Pubkey::from_str("pc mint address").unwrap(),
    user_token_coin: Pubkey::from_str("user token coin address").unwrap(),
    user_token_pc: Pubkey::from_str("user token pc address").unwrap(),
    init_coin_amount: 100000u64,
    init_pc_amount: 100000u64,
    open_time: 0,
};
let instruction = amm_cli::process_amm_commands(subcmd, &config).unwrap();
```