use crate::{cpswap_instructions, cpswap_utils, decode_cpswap_ix_event};
use anyhow::Result;
use clap::Parser;
use common::{common_types, common_utils, rpc, token};
use rand::rngs::OsRng;
use solana_client::{
    rpc_client::RpcClient,
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};
use std::sync::Arc;

#[derive(Debug, Parser)]
pub enum CpSwapCommands {
    CreatePool {
        /// User's token0.
        /// The token's mint must match with the pool's mint0.
        #[clap(long)]
        user_token0: Pubkey,
        /// User's token1.
        /// The token's mint must match with the pool's mint1.
        #[clap(long)]
        user_token1: Pubkey,
        /// The cp pool config account stored the fee infos.
        #[clap(short, long)]
        amm_config: Pubkey,
        /// The amount to init of toke0.
        /// Amount can't be 0.
        #[clap(long)]
        init_amount_0: u64,
        /// The amount to init of toke1.
        /// Ammount can't be 0.
        #[clap(long)]
        init_amount_1: u64,
        /// The time of the pool is allowed to swap.
        /// If time is less than or equal to the on-chain timestamp, it will be set to the on-chain timestamp + 1.
        #[clap(short, long, default_value_t = 0)]
        open_time: u64,
        /// The pool id is random or not.
        #[clap(short, long, action)]
        random_pool: bool,
    },
    Deposit {
        /// The specified pool of the assets deposite to
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The specified token0 of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token0: Option<Pubkey>,
        /// The specified token1 of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token1: Option<Pubkey>,
        /// The specified lp token of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token_lp: Option<Pubkey>,
        /// The amount of the specified token to deposit.
        #[clap(short, long)]
        amount_specified: u64,
        /// Indicates which token is specified of the `amount_specified`.
        #[clap(short, long, action)]
        base_token1: bool,
    },
    Withdraw {
        /// The specified pool of the assets withdraw from.
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The specified lp token of the user withdraw.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        withdraw_token_lp: Option<Pubkey>,
        /// The specified token0 of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token0: Option<Pubkey>,
        /// The specified token1 of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token1: Option<Pubkey>,
        /// The amount of liquidity to withdraw.
        #[clap(short, long)]
        input_lp_amount: u64,
    },
    Swap {
        /// The specified pool of trading.
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The token of user want to swap from.
        #[clap(long)]
        user_input_token: Pubkey,
        /// The token of user want to swap to.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        user_output_token: Option<Pubkey>,
        /// The amount specified of user want to swap from or to token
        #[clap(short, long)]
        amount_specified: u64,
        /// The amount specified is output_token or not.
        #[clap(short, long, action)]
        base_out: bool,
    },
    FetchPool {
        /// The specified pool to fetch. If none is given, fetch pools by mint0 and mint1.
        /// If the pool_id is specified, mint0 and mint1 will be ignored.
        #[clap(long)]
        pool_id: Option<Pubkey>,
        /// Fetch pools by specified mint0.
        #[clap(long)]
        mint0: Option<Pubkey>,
        /// Fetch pools by specified mint1.
        #[clap(long)]
        mint1: Option<Pubkey>,
    },
    FetchConfig {
        /// The specified amm config to fetch. If none is given, fetch all configs.
        #[clap(long)]
        amm_config: Option<Pubkey>,
    },
    DecodeIx {
        // Instruction hex data
        #[clap(short, long)]
        ix_data: String,
    },
    DecodeEvent {
        // Program event log
        #[clap(short, long)]
        event_data: String,
    },
}

pub fn process_cpswap_commands(
    command: CpSwapCommands,
    config: &common_types::CommonConfig,
    signing_keypairs: &mut Vec<Arc<dyn Signer>>,
) -> Result<Option<Vec<Instruction>>> {
    let rpc_client = RpcClient::new(config.cluster().url());
    let wallet_keypair = common_utils::read_keypair_file(&config.wallet())?;
    let payer_pubkey = wallet_keypair.pubkey();
    let payer: Arc<dyn Signer> = Arc::new(wallet_keypair);
    if !signing_keypairs.contains(&payer) {
        signing_keypairs.push(payer);
    }

    match command {
        CpSwapCommands::CreatePool {
            user_token0,
            user_token1,
            amm_config,
            init_amount_0,
            init_amount_1,
            open_time,
            random_pool,
        } => {
            let load_pubkeys = vec![user_token0, user_token1];
            let rsps = rpc_client.get_multiple_accounts(&load_pubkeys)?;
            let token0_program = rsps[0].as_ref().unwrap().owner;
            let token1_program = rsps[1].as_ref().unwrap().owner;
            let user_token0_account =
                common_utils::unpack_token(&rsps[0].as_ref().unwrap().data).unwrap();
            let user_token1_account =
                common_utils::unpack_token(&rsps[1].as_ref().unwrap().data).unwrap();

            let (
                user_token0,
                user_token1,
                mint0,
                mint1,
                token0_program,
                token1_program,
                init_amount_0,
                init_amount_1,
            ) = if user_token0_account.base.mint > user_token1_account.base.mint {
                println!("Flip user_token0, init_amount_0 and user_token1, init_amount_1 because mint0 be less than mint1");
                (
                    user_token1,
                    user_token0,
                    user_token1_account.base.mint,
                    user_token0_account.base.mint,
                    token1_program,
                    token0_program,
                    init_amount_1,
                    init_amount_0,
                )
            } else {
                (
                    user_token0,
                    user_token1,
                    user_token0_account.base.mint,
                    user_token1_account.base.mint,
                    token0_program,
                    token1_program,
                    init_amount_0,
                    init_amount_1,
                )
            };

            let random_pool_id = if random_pool {
                let random_pool_keypair = Keypair::generate(&mut OsRng);
                let random_pool_id = random_pool_keypair.pubkey();
                let signer: Arc<dyn Signer> = Arc::new(random_pool_keypair);
                if !signing_keypairs.contains(&signer) {
                    println!("random_pool_id:{}", random_pool_id);
                    signing_keypairs.push(signer);
                }
                Some(random_pool_id)
            } else {
                None
            };

            let initialize_pool_instr = cpswap_instructions::initialize_pool_instr(
                &config,
                mint0,
                mint1,
                token0_program,
                token1_program,
                user_token0,
                user_token1,
                raydium_cp_swap::create_pool_fee_reveiver::ID,
                amm_config,
                random_pool_id,
                init_amount_0,
                init_amount_1,
                open_time,
            )?;
            return Ok(Some(initialize_pool_instr));
        }
        CpSwapCommands::Deposit {
            pool_id,
            deposit_token0,
            deposit_token1,
            recipient_token_lp,
            amount_specified,
            base_token1,
        } => {
            let base_token0 = !base_token1;
            let result = cpswap_utils::add_liquidity_calculate(
                &rpc_client,
                pool_id,
                amount_specified,
                config.slippage(),
                base_token0,
            )?;
            let deposit_token0 = if let Some(deposit_token0) = deposit_token0 {
                deposit_token0
            } else {
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &result.mint0,
                    &result.mint0_token_program,
                )
            };
            let deposit_token1 = if let Some(deposit_token1) = deposit_token1 {
                deposit_token1
            } else {
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &result.mint1,
                    &result.mint1_token_program,
                )
            };

            let mut instructions = Vec::new();
            let recipient_token_lp = if let Some(recipient_token_lp) = recipient_token_lp {
                recipient_token_lp
            } else {
                let create_user_token_lp_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.mintlp,
                    &payer_pubkey,
                    None,
                );
                instructions.extend(create_user_token_lp_instr);

                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.mintlp,
                )
            };

            let deposit_instr = cpswap_instructions::deposit_instr(
                &config,
                pool_id,
                result.mint0,
                result.mint1,
                result.mintlp,
                result.vault0,
                result.vault1,
                deposit_token0,
                deposit_token1,
                recipient_token_lp,
                result.lp_token_amount,
                result.amount_0,
                result.amount_1,
            )?;
            instructions.extend(deposit_instr);
            return Ok(Some(instructions));
        }
        CpSwapCommands::Withdraw {
            pool_id,
            withdraw_token_lp,
            recipient_token0,
            recipient_token1,
            input_lp_amount,
        } => {
            let result = cpswap_utils::remove_liquidity_calculate(
                &rpc_client,
                pool_id,
                input_lp_amount,
                config.slippage(),
            )?;
            let withdraw_token_lp = if let Some(withdraw_token_lp) = withdraw_token_lp {
                withdraw_token_lp
            } else {
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.mintlp,
                )
            };

            let mut instructions = Vec::new();
            let recipient_token0 = if let Some(recipient_token0) = recipient_token0 {
                recipient_token0
            } else {
                // mint0 maybe token22
                let create_user_token0_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.mint0,
                    &payer_pubkey,
                    Some(&result.mint0_token_program),
                );
                instructions.extend(create_user_token0_instr);

                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &result.mint0,
                    &result.mint0_token_program,
                )
            };
            let recipient_token1 = if let Some(recipient_token1) = recipient_token1 {
                recipient_token1
            } else {
                // mint1 maybe token22
                let create_user_token1_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.mint1,
                    &payer_pubkey,
                    Some(&result.mint1_token_program),
                );
                instructions.extend(create_user_token1_instr);

                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &result.mint1,
                    &result.mint1_token_program,
                )
            };

            let withdraw_instr = cpswap_instructions::withdraw_instr(
                &config,
                pool_id,
                result.mint0,
                result.mint1,
                result.mintlp,
                result.vault0,
                result.vault1,
                recipient_token0,
                recipient_token1,
                withdraw_token_lp,
                result.lp_token_amount,
                result.amount_0,
                result.amount_1,
            )?;
            instructions.extend(withdraw_instr);
            return Ok(Some(instructions));
        }
        CpSwapCommands::Swap {
            pool_id,
            user_input_token,
            user_output_token,
            amount_specified,
            base_out,
        } => {
            let base_in = !base_out;
            let result = cpswap_utils::swap_calculate(
                &rpc_client,
                pool_id,
                user_input_token,
                amount_specified,
                config.slippage(),
                base_in,
            )?;

            let mut instructions = Vec::new();
            let user_output_token = if let Some(user_output_token) = user_output_token {
                user_output_token
            } else {
                let create_user_output_token_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.output_mint,
                    &payer_pubkey,
                    Some(&result.output_token_program),
                );
                instructions.extend(create_user_output_token_instr);

                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &result.output_mint,
                    &result.output_token_program,
                )
            };

            let swap_instruction = if base_in {
                cpswap_instructions::swap_base_input_instr(
                    &config,
                    pool_id,
                    result.pool_config,
                    result.pool_observation,
                    result.user_input_token,
                    user_output_token,
                    result.input_vault,
                    result.output_vault,
                    result.input_mint,
                    result.output_mint,
                    result.input_token_program,
                    result.output_token_program,
                    result.amount_specified,
                    result.other_amount_threshold,
                )?
            } else {
                cpswap_instructions::swap_base_output_instr(
                    &config,
                    pool_id,
                    result.pool_config,
                    result.pool_observation,
                    result.user_input_token,
                    user_output_token,
                    result.input_vault,
                    result.output_vault,
                    result.input_mint,
                    result.output_mint,
                    result.input_token_program,
                    result.output_token_program,
                    result.amount_specified,
                    result.other_amount_threshold,
                )?
            };
            instructions.extend(swap_instruction);
            return Ok(Some(instructions));
        }
        CpSwapCommands::FetchPool {
            pool_id,
            mint0,
            mint1,
        } => {
            if let Some(pool_id) = pool_id {
                // fetch specified pool
                let pool_state = rpc::get_anchor_account::<raydium_cp_swap::states::PoolState>(
                    &rpc_client,
                    &pool_id,
                )
                .unwrap()
                .unwrap();
                println!("{:#?}", pool_state);
            } else {
                // fetch pool by filters
                let pool_len = raydium_cp_swap::states::PoolState::LEN as u64;
                let filters = match (mint0, mint1) {
                    (None, None) => Some(vec![RpcFilterType::DataSize(pool_len)]),
                    (Some(mint0), None) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 5 * 32,
                            &mint0.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                    (None, Some(mint1)) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 6 * 32,
                            &mint1.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                    (Some(mint0), Some(mint1)) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 5 * 32,
                            &mint0.to_bytes(),
                        )),
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 6 * 32,
                            &mint1.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                };
                let pools = rpc::get_program_accounts_with_filters(
                    &rpc_client,
                    config.cp_program(),
                    filters,
                )
                .unwrap();
                for pool in pools {
                    println!("pool_id:{}", pool.0);
                    println!(
                        "{:#?}",
                        common_utils::deserialize_anchor_account::<
                            raydium_cp_swap::states::PoolState,
                        >(&pool.1)
                    );
                }
            }
            return Ok(None);
        }
        CpSwapCommands::FetchConfig { amm_config } => {
            let mut config_info = "".to_string();
            if let Some(amm_config) = amm_config {
                // fetch specified amm_config
                let amm_config_state =
                    rpc::get_anchor_account::<raydium_cp_swap::states::AmmConfig>(
                        &rpc_client,
                        &amm_config,
                    )
                    .unwrap()
                    .unwrap();
                // println!("{:#?}", amm_config_state);
                let trade_fee_rate =
                    amm_config_state.trade_fee_rate as f64 / common_types::TEN_THOUSAND as f64;
                let protocol_fee_rate =
                    amm_config_state.protocol_fee_rate as f64 / common_types::TEN_THOUSAND as f64;
                let fund_fee_rate =
                    amm_config_state.fund_fee_rate as f64 / common_types::TEN_THOUSAND as f64;
                let string = format!(
                    "amm_config:{}, index:{}, trade: {:.2}%, protocol: {:.2}%, fund: {:.2}% \n",
                    amm_config,
                    amm_config_state.index,
                    trade_fee_rate,
                    protocol_fee_rate,
                    fund_fee_rate
                );
                config_info.push_str(string.as_str());
            } else {
                // fetch all amm_config
                let amm_configs = rpc::get_program_accounts_with_filters(
                    &rpc_client,
                    config.cp_program(),
                    Some(vec![RpcFilterType::DataSize(
                        raydium_cp_swap::states::AmmConfig::LEN as u64,
                    )]),
                )
                .unwrap();
                for amm_config in amm_configs {
                    let amm_config_state = common_utils::deserialize_anchor_account::<
                        raydium_cp_swap::states::AmmConfig,
                    >(&amm_config.1)
                    .unwrap();
                    // println!("{:#?}", amm_config_state);
                    let trade_fee_rate =
                        amm_config_state.trade_fee_rate as f64 / common_types::TEN_THOUSAND as f64;
                    let protocol_fee_rate = amm_config_state.protocol_fee_rate as f64
                        / common_types::TEN_THOUSAND as f64;
                    let fund_fee_rate =
                        amm_config_state.fund_fee_rate as f64 / common_types::TEN_THOUSAND as f64;
                    let string = format!(
                        "amm_config:{}, index:{}, trade: {:.2}%, protocol: {:.2}%, fund: {:.2}% \n",
                        amm_config.0,
                        amm_config_state.index,
                        trade_fee_rate,
                        protocol_fee_rate,
                        fund_fee_rate
                    );
                    config_info.push_str(string.as_str());
                }
            }
            if !config_info.is_empty() {
                println!("{}", config_info);
            }
            return Ok(None);
        }
        CpSwapCommands::DecodeIx { ix_data } => {
            decode_cpswap_ix_event::handle_program_instruction(
                ix_data.as_str(),
                common_types::InstructionDecodeType::BaseHex,
            )?;
            return Ok(None);
        }
        CpSwapCommands::DecodeEvent { event_data } => {
            decode_cpswap_ix_event::handle_program_event(event_data.as_str(), false)?;
            return Ok(None);
        }
    }
}
