use crate::{amm_instructions, amm_utils, decode_amm_ix_event, openbook};
use anyhow::Ok;
use anyhow::Result;
use clap::Parser;
use common::{common_types, common_utils, rpc, token};
use raydium_amm::state::Loadable;
use solana_client::{
    rpc_client::RpcClient,
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Signer};

#[derive(Debug, Parser)]
pub enum AmmCommands {
    CreatePool {
        /// The amm associated with openbook market
        #[clap(short, long)]
        market: Pubkey,
        /// The openbook market's coin_mint
        #[clap(long)]
        coin_mint: Pubkey,
        /// The openbook market's pc_mint
        #[clap(long)]
        pc_mint: Pubkey,
        /// User's token coin.
        /// The token's mint must match with the market's coin_mint.
        #[clap(long)]
        user_token_coin: Pubkey,
        /// User's token pc.
        /// The token's mint must match with the market's pc_mint.
        #[clap(long)]
        user_token_pc: Pubkey,
        /// The amount to init of toke coin.
        /// Ammount can't be 0.
        #[clap(long)]
        init_coin_amount: u64,
        /// The amount to init of toke pc.
        /// Ammount can't be 0.
        #[clap(long)]
        init_pc_amount: u64,
        /// The time of the pool is allowed to swap.
        /// If time is less than or equal to the on-chain timestamp, it will be set to swap immediately.
        #[arg(short, long, default_value_t = 0)]
        open_time: u64,
    },
    Deposit {
        /// The specified pool of the assets deposite to
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The specified token coin of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token_coin: Option<Pubkey>,
        /// The specified token pc of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token_pc: Option<Pubkey>,
        /// The specified lp token of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token_lp: Option<Pubkey>,
        /// The amount of the specified token to deposit.
        amount_specified: u64,
        /// The amount of the other side to be calculated may be less than expected due to price fluctuations.
        /// It's necessary to add an optional parameter to limit the minimum amount of the other side.
        #[clap(short, long, action)]
        another_min_limit: bool,
        /// Indicates which token is specified of the `amount_specified`.
        #[arg(short, long, action)]
        base_coin: bool,
    },
    Withdraw {
        /// The specified pool of the assets withdraw from.
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The specified lp token of the user withdraw.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        withdraw_token_lp: Option<Pubkey>,
        /// The specified token coin of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token_coin: Option<Pubkey>,
        /// The specified token pc of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token_pc: Option<Pubkey>,
        /// The amount of liquidity to withdraw.
        #[clap(short, long)]
        input_lp_amount: u64,
        /// The amount of both tokens to be calculated though `input_lp_amount` may be less than expected due to price fluctuations.
        /// It's necessary to add an optional parameter to limit the minimum amount of the tokens.
        #[clap(short, long, action)]
        slippage_limit: bool,
    },
    Swap {
        /// The specified pool of trading.
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The token of user want to swap to.
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
        /// If the pool_id is specified, coin_mint and pc_mint will be ignored.
        #[clap(long)]
        pool_id: Option<Pubkey>,
        /// Fetch pools by specified coin_mint.
        #[clap(long)]
        coin_mint: Option<Pubkey>,
        /// Fetch pools by specified pc_mint.
        #[clap(long)]
        pc_mint: Option<Pubkey>,
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
    SimulateInfo {
        #[clap(short, long)]
        pool_id: Pubkey,
    },
}
pub fn process_amm_commands(
    command: AmmCommands,
    config: &common_types::CommonConfig,
) -> Result<Option<Vec<Instruction>>> {
    let rpc_client = RpcClient::new(config.cluster().url());
    let wallet_keypair = common_utils::read_keypair_file(&config.wallet())?;
    let payer_pubkey = wallet_keypair.pubkey();

    match command {
        AmmCommands::CreatePool {
            market,
            coin_mint,
            pc_mint,
            user_token_coin,
            user_token_pc,
            init_coin_amount,
            init_pc_amount,
            open_time,
        } => {
            let market_keys =
                openbook::get_keys_for_market(&rpc_client, &config.openbook_program(), &market)
                    .unwrap();
            assert_eq!(coin_mint, *market_keys.coin_mint);
            assert_eq!(pc_mint, *market_keys.pc_mint);
            let amm_keys = amm_utils::get_amm_pda_keys(
                &config.amm_program(),
                &config.openbook_program(),
                &market,
                &coin_mint,
                &pc_mint,
            )?;
            let instruction = amm_instructions::initialize_amm_pool(
                &config.amm_program(),
                &amm_keys,
                &raydium_amm::processor::config_feature::create_pool_fee_address::id(),
                &wallet_keypair.pubkey(),
                &user_token_coin,
                &user_token_pc,
                &spl_associated_token_account::get_associated_token_address(
                    &wallet_keypair.pubkey(),
                    &amm_keys.amm_lp_mint,
                ),
                open_time,
                init_pc_amount,
                init_coin_amount,
            )?;
            return Ok(Some(vec![instruction]));
        }
        AmmCommands::Deposit {
            pool_id,
            deposit_token_coin,
            deposit_token_pc,
            recipient_token_lp,
            amount_specified,
            another_min_limit,
            base_coin,
        } => {
            let base_side = if base_coin { 0 } else { 1 };
            let result = amm_utils::calculate_deposit_info(
                &rpc_client,
                config.amm_program(),
                pool_id,
                amount_specified,
                another_min_limit,
                config.slippage(),
                base_side,
            )
            .unwrap();
            let deposit_token_coin = if let Some(deposit_token_coin) = deposit_token_coin {
                deposit_token_coin
            } else {
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.amm_coin_mint,
                )
            };
            let deposit_token_pc = if let Some(deposit_token_pc) = deposit_token_pc {
                deposit_token_pc
            } else {
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.amm_pc_mint,
                )
            };

            let mut instructions = Vec::new();
            let recipient_token_lp = if let Some(recipient_token_lp) = recipient_token_lp {
                recipient_token_lp
            } else {
                // create ata token lp or not
                let create_user_token_lp_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.amm_lp_mint,
                    &payer_pubkey,
                    None,
                );
                instructions.extend(create_user_token_lp_instr);

                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.amm_lp_mint,
                )
            };

            let instruction = raydium_amm::instruction::deposit(
                &config.amm_program(),
                &result.pool_id,
                &result.amm_authority,
                &result.amm_open_orders,
                &result.amm_target_orders,
                &result.amm_lp_mint,
                &result.amm_coin_vault,
                &result.amm_pc_vault,
                &result.market,
                &result.market_event_queue,
                &deposit_token_coin,
                &deposit_token_pc,
                &recipient_token_lp,
                &wallet_keypair.pubkey(),
                result.max_coin_amount,
                result.max_pc_amount,
                base_side,
                result.another_min_amount,
            )?;
            instructions.extend(vec![instruction]);

            return Ok(Some(instructions));
        }
        AmmCommands::Withdraw {
            pool_id,
            withdraw_token_lp,
            recipient_token_coin,
            recipient_token_pc,
            input_lp_amount,
            slippage_limit,
        } => {
            let result = amm_utils::calculate_withdraw_info(
                &rpc_client,
                config.amm_program(),
                pool_id,
                input_lp_amount,
                if slippage_limit {
                    Some(config.slippage())
                } else {
                    None
                },
            )
            .unwrap();
            let withdraw_token_lp = if let Some(withdraw_token_lp) = withdraw_token_lp {
                withdraw_token_lp
            } else {
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.amm_lp_mint,
                )
            };
            let mut instructions = Vec::new();
            let recipient_token_coin = if let Some(recipient_token_coin) = recipient_token_coin {
                recipient_token_coin
            } else {
                // create ata token coin or not
                let create_user_token_coin_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.amm_coin_mint,
                    &payer_pubkey,
                    None,
                );

                instructions.extend(create_user_token_coin_instr);
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.amm_coin_mint,
                )
            };
            let recipient_token_pc = if let Some(recipient_token_pc) = recipient_token_pc {
                recipient_token_pc
            } else {
                // create ata token pc or not
                let create_user_token_pc_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.amm_pc_mint,
                    &payer_pubkey,
                    None,
                );

                instructions.extend(create_user_token_pc_instr);
                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.amm_pc_mint,
                )
            };
            let instruction = raydium_amm::instruction::withdraw(
                &config.amm_program(),
                &result.pool_id,
                &result.amm_authority,
                &result.amm_open_orders,
                &result.amm_target_orders,
                &result.amm_lp_mint,
                &result.amm_coin_vault,
                &result.amm_pc_vault,
                &result.market_program,
                &result.market,
                &result.market_coin_vault,
                &result.market_pc_vault,
                &result.market_vault_signer,
                &withdraw_token_lp,
                &recipient_token_coin,
                &recipient_token_pc,
                &wallet_keypair.pubkey(),
                &result.market_event_queue,
                &result.market_bids,
                &result.market_asks,
                None,
                input_lp_amount,
                result.receive_min_coin_amount,
                result.receive_min_pc_amount,
            )?;
            instructions.extend(vec![instruction]);

            return Ok(Some(instructions));
        }
        AmmCommands::Swap {
            pool_id,
            user_input_token,
            user_output_token,
            amount_specified,
            base_out,
        } => {
            let base_in = !base_out;
            let result = amm_utils::calculate_swap_info(
                &rpc_client,
                config.amm_program(),
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
                // create output token or not
                let create_user_output_token_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.output_mint,
                    &payer_pubkey,
                    None,
                );
                instructions.extend(create_user_output_token_instr);

                spl_associated_token_account::get_associated_token_address(
                    &payer_pubkey,
                    &result.output_mint,
                )
            };

            let swap_instruction = if base_in {
                raydium_amm::instruction::swap_base_in(
                    &config.amm_program(),
                    &result.pool_id,
                    &result.amm_authority,
                    &result.amm_open_orders,
                    &result.amm_coin_vault,
                    &result.amm_pc_vault,
                    &result.market_program,
                    &result.market,
                    &result.market_bids,
                    &result.market_asks,
                    &result.market_event_queue,
                    &result.market_coin_vault,
                    &result.market_pc_vault,
                    &result.market_vault_signer,
                    &user_input_token,
                    &user_output_token,
                    &wallet_keypair.pubkey(),
                    result.amount_specified,
                    result.other_amount_threshold,
                )?
            } else {
                raydium_amm::instruction::swap_base_out(
                    &config.amm_program(),
                    &result.pool_id,
                    &result.amm_authority,
                    &result.amm_open_orders,
                    &result.amm_coin_vault,
                    &result.amm_pc_vault,
                    &result.market_program,
                    &result.market,
                    &result.market_bids,
                    &result.market_asks,
                    &result.market_event_queue,
                    &result.market_coin_vault,
                    &result.market_pc_vault,
                    &result.market_vault_signer,
                    &user_input_token,
                    &user_output_token,
                    &wallet_keypair.pubkey(),
                    result.other_amount_threshold,
                    result.amount_specified,
                )?
            };
            instructions.extend(vec![swap_instruction]);
            return Ok(Some(instructions));
        }
        AmmCommands::FetchPool {
            pool_id,
            coin_mint,
            pc_mint,
        } => {
            if pool_id.is_some() {
                // fetch specified pool
                let amm_data = rpc::get_account(&rpc_client, &pool_id.unwrap())
                    .unwrap()
                    .unwrap();
                let pool_state = raydium_amm::state::AmmInfo::load_from_bytes(&amm_data).unwrap();
                println!("{:#?}", pool_state);
            } else {
                // fetch pool by filters
                let pool_len = core::mem::size_of::<raydium_amm::state::AmmInfo>() as u64;
                let filters = match (coin_mint, pc_mint) {
                    (None, None) => Some(vec![RpcFilterType::DataSize(pool_len)]),
                    (Some(coin_mint), None) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            400,
                            &coin_mint.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                    (None, Some(pc_mint)) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(432, &pc_mint.to_bytes())),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                    (Some(coin_mint), Some(pc_mint)) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            400,
                            &coin_mint.to_bytes(),
                        )),
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(432, &pc_mint.to_bytes())),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                };
                let pools = rpc::get_program_accounts_with_filters(
                    &rpc_client,
                    config.amm_program(),
                    filters,
                )
                .unwrap();
                for pool in pools {
                    println!("pool_id:{}", pool.0);
                    println!(
                        "{:#?}",
                        raydium_amm::state::AmmInfo::load_from_bytes(&pool.1.data)
                    );
                }
            }
            return Ok(None);
        }
        AmmCommands::DecodeIx { ix_data } => {
            decode_amm_ix_event::handle_program_instruction(
                ix_data.as_str(),
                common_types::InstructionDecodeType::BaseHex,
            )?;
            return Ok(None);
        }
        AmmCommands::DecodeEvent { event_data } => {
            decode_amm_ix_event::handle_program_event(event_data.as_str(), false)?;
            return Ok(None);
        }
        AmmCommands::SimulateInfo { pool_id } => {
            let amm_keys = amm_utils::load_amm_keys(&rpc_client, &config.amm_program(), &pool_id)?;
            let market_state = openbook::get_keys_for_market(
                &rpc_client,
                &amm_keys.market_program,
                &amm_keys.market,
            )
            .unwrap();

            let simulate_instr = raydium_amm::instruction::simulate_get_pool_info(
                &config.amm_program(),
                &pool_id,
                &amm_keys.amm_authority,
                &amm_keys.amm_open_order,
                &amm_keys.amm_coin_vault,
                &amm_keys.amm_pc_vault,
                &amm_keys.amm_lp_mint,
                &amm_keys.market,
                &market_state.event_q,
                None,
            )?;
            return Ok(Some(vec![simulate_instr]));
        }
    }
}
