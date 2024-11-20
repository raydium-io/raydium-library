#![allow(dead_code)]

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
    pub command_override: common_types::CommonConfig,
    #[clap(subcommand)]
    pub command: Command,
}

pub fn entry(opts: Opts) -> Result<()> {
    // default config
    let mut config = common_types::CommonConfig::default();
    // config file override
    config.file_override().unwrap();
    // config command override
    let command_override = opts.command_override;
    config.command_override(command_override);

    let payer = common_utils::read_keypair_file(&config.wallet())?;
    let fee_payer = payer.pubkey();
    let mut signing_keypairs: Vec<Arc<dyn Signer>> = Vec::new();
    let payer: Arc<dyn Signer> = Arc::new(payer);
    if !signing_keypairs.contains(&payer) {
        signing_keypairs.push(payer);
    }

    let instructions = match opts.command {
        Command::CPSWAP { subcmd } => {
            cpswap_cli::process_cpswap_commands(subcmd, &config, &mut signing_keypairs).unwrap()
        }
        Command::AMM { subcmd } => amm_cli::process_amm_commands(subcmd, &config).unwrap(),
        Command::CLMM { subcmd } => {
            clmm_cli::process_clmm_commands(subcmd, &config, &mut signing_keypairs).unwrap()
        }
    };
    match instructions {
        Some(instructions) => {
            // build txn
            let rpc_client = RpcClient::new(config.cluster().url());
            let txn =
                rpc::build_txn(&rpc_client, &instructions, &fee_payer, &signing_keypairs).unwrap();
            if config.simulate() {
                let sig = rpc::simulate_transaction(
                    &rpc_client,
                    &txn,
                    false,
                    CommitmentConfig::confirmed(),
                );
                println!("{:#?}", sig);
            } else {
                //  send txn
                let sig = rpc::send_txn(&rpc_client, &txn, true);
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
