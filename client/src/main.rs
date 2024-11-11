#![allow(dead_code)]

use anyhow::{Ok, Result};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signer::Signer};
use std::sync::Arc;

use raydium_library::{
    amm::{self, AmmCommands},
    clmm::{self, ClmmCommands},
    common::{self, types::CommonConfig},
    cpswap::{self, CpSwapCommands},
};
/// commands
#[derive(Debug, Parser)]
pub enum Command {
    CPSWAP {
        #[clap(subcommand)]
        subcmd: CpSwapCommands,
    },
    CLMM {
        #[clap(subcommand)]
        subcmd: ClmmCommands,
    },
    AMM {
        #[clap(subcommand)]
        subcmd: AmmCommands,
    },
}

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(flatten)]
    pub command_override: CommonConfig,
    #[clap(subcommand)]
    pub command: Command,
}

pub fn entry(opts: Opts) -> Result<()> {
    // default config
    let mut config = common::CommonConfig::default();
    // println!("{:#?}", config);
    // config file override
    config.file_override().unwrap();
    // println!("{:#?}", config);
    // config command override
    let command_override = opts.command_override;
    config.command_override(command_override);
    // println!("{:#?}", config);

    let payer = common::utils::read_keypair_file(&config.wallet())?;
    let fee_payer = payer.pubkey();
    let mut signing_keypairs: Vec<Arc<dyn Signer>> = Vec::new();
    let payer: Arc<dyn Signer> = Arc::new(payer);
    if !signing_keypairs.contains(&payer) {
        signing_keypairs.push(payer);
    }

    let instructions = match opts.command {
        Command::CPSWAP { subcmd } => {
            cpswap::process_cpswap_commands(subcmd, &config, &mut signing_keypairs).unwrap()
        }
        Command::AMM { subcmd } => amm::process_amm_commands(subcmd, &config).unwrap(),
        Command::CLMM { subcmd } => {
            clmm::process_clmm_commands(subcmd, &config, &mut signing_keypairs).unwrap()
        }
    };
    match instructions {
        Some(instructions) => {
            // build txn
            let rpc_client = RpcClient::new(config.cluster().url());
            let txn = common::build_txn(&rpc_client, &instructions, &fee_payer, &signing_keypairs)
                .unwrap();
            // println!("{:#?}", txn);
            if config.simulate() {
                let sig = common::simulate_transaction(
                    &rpc_client,
                    &txn,
                    false,
                    CommitmentConfig::confirmed(),
                );
                println!("{:#?}", sig);
            } else {
                //  send txn
                let sig = common::send_txn(&rpc_client, &txn, true);
                println!("{:#?}", sig);
            }
        }
        None => {
            // do nothing
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    entry(Opts::parse())
}
