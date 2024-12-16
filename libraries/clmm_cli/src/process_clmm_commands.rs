use crate::{clmm_instructions, clmm_utils, decode_clmm_ix_event};
use anyhow::Result;
use clap::Parser;
use common::{common_types, common_utils, rpc, token};
use rand::rngs::OsRng;
use solana_client::{
    rpc_client::RpcClient,
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    signer::keypair::Keypair,
};
use std::sync::Arc;

#[derive(Debug, Parser)]
pub enum ClmmCommands {
    CreatePool {
        /// The pool's mint0
        #[arg(long)]
        mint0: Pubkey,
        /// The pool's mint1
        #[arg(long)]
        mint1: Pubkey,
        /// The clmm pool config account stored tick_spaceing and the fee infos.
        #[arg(short, long)]
        amm_config: Pubkey,
        /// The float price of token mint0 relative to token mint1
        #[arg(long)]
        price: f64,
        /// The time of the pool is allowed to swap.
        #[arg(short, long, default_value_t = 0)]
        open_time: u64,
    },
    OpenPosition {
        /// The specified pool of the assets deposite to
        #[arg(short, long)]
        pool_id: Pubkey,
        /// The specified token0 of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token0: Option<Pubkey>,
        /// The specified token1 of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token1: Option<Pubkey>,
        /// The float price of token mint0 relative to token mint1
        /// The position lower price
        #[arg(long)]
        tick_lower_price: f64,
        /// The float price of token mint0 relative to token mint1
        /// The position upper price
        #[arg(long)]
        tick_upper_price: f64,
        /// The amount of the specified token to deposit.
        #[arg(long)]
        amount_specified: u64,
        /// Indicates which token is specified of the `amount_specified`.
        #[clap(short, long, action)]
        base_token1: bool,
        /// Whether need to create metadata for the NFT mint of the position.
        #[arg(short, long, action)]
        without_metadata: bool,
        /// The default is token_2022 NFT. If specified, create mpl NFT
        #[arg(short, long, action)]
        traditional_nft: bool,
    },
    IncreaseLiquidity {
        /// The specified pool of the assets deposite to
        #[arg(short, long)]
        pool_id: Pubkey,
        /// The specified token0 of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token0: Option<Pubkey>,
        /// The specified token1 of the user deposit.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        deposit_token1: Option<Pubkey>,
        /// The float price of token mint0 relative to token mint1
        /// The position lower price
        tick_lower_price: f64,
        /// The float price of token mint0 relative to token mint1
        /// The position upper price
        tick_upper_price: f64,
        /// The amount of the specified token to deposit.
        #[arg(long)]
        amount_specified: u64,
        /// Indicates which token is specified of the `amount_specified`.
        #[clap(short, long, action)]
        base_token1: bool,
    },
    DecreaseLiquidity {
        /// The specified pool of the assets withdraw from.
        #[clap(short, long)]
        pool_id: Pubkey,
        /// The specified token0 of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token0: Option<Pubkey>,
        /// The specified token1 of the user will receive.
        /// If none is given, the account will be ATA account.
        #[clap(long)]
        recipient_token1: Option<Pubkey>,
        /// The float price of token mint0 relative to token mint1
        /// The position lower price
        tick_lower_price: f64,
        /// The float price of token mint0 relative to token mint1
        /// The position upper price
        tick_upper_price: f64,
        /// The amount of the specified token to withdraw.
        #[arg(long)]
        amount_specified: u64,
        /// Indicates which token is specified of the `amount_specified`.
        #[clap(short, long, action)]
        base_token1: bool,
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
        /// The amount specified of user want to swap from or to token.
        #[clap(short, long)]
        amount_specified: u64,
        /// The float price of the pool that can be swaped to.
        #[clap(short, long)]
        limit_price: Option<f64>,
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
        /// The specified clmm config to fetch. If none is given, fetch all configs.
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

pub fn process_clmm_commands(
    command: ClmmCommands,
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
        ClmmCommands::CreatePool {
            mint0,
            mint1,
            amm_config,
            price,
            open_time,
        } => {
            let result = clmm_utils::create_pool_price(&rpc_client, mint0, mint1, price)?;
            let create_pool_instr = clmm_instructions::create_pool_instr(
                &config,
                amm_config,
                result.mint0,
                result.mint1,
                result.mint0_token_program,
                result.mint1_token_program,
                result.sqrt_price_x64,
                open_time,
            )?;
            return Ok(Some(create_pool_instr));
        }
        ClmmCommands::OpenPosition {
            pool_id,
            deposit_token0,
            deposit_token1,
            tick_lower_price,
            tick_upper_price,
            amount_specified,
            base_token1,
            without_metadata,
            traditional_nft,
        } => {
            let base_token0 = !base_token1;
            let with_metadata = !without_metadata;
            let result = clmm_utils::calculate_liquidity_change(
                &rpc_client,
                pool_id,
                tick_lower_price,
                tick_upper_price,
                amount_specified,
                config.slippage(),
                false,
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

            // load position
            let (_nft_tokens, positions) = clmm_utils::get_nft_accounts_and_positions_by_owner(
                &rpc_client,
                &payer_pubkey,
                &config.clmm_program(),
            );
            let rsps = rpc_client.get_multiple_accounts(&positions)?;
            let mut user_positions = Vec::new();
            for rsp in rsps {
                match rsp {
                    None => continue,
                    Some(rsp) => {
                        let position = common_utils::deserialize_anchor_account::<
                            raydium_amm_v3::states::PersonalPositionState,
                        >(&rsp)?;
                        user_positions.push(position);
                    }
                }
            }
            let mut find_position = raydium_amm_v3::states::PersonalPositionState::default();
            for position in user_positions {
                if position.pool_id == pool_id
                    && position.tick_lower_index == result.tick_lower_index
                    && position.tick_upper_index == result.tick_upper_index
                {
                    find_position = position.clone();
                }
            }
            if find_position.nft_mint == Pubkey::default() {
                let tickarray_bitmap_extension = Pubkey::find_program_address(
                    &[
                        raydium_amm_v3::states::POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(),
                        pool_id.to_bytes().as_ref(),
                    ],
                    &config.clmm_program(),
                )
                .0;
                // personal position not exist
                // new nft mint
                let nft_mint = Keypair::generate(&mut OsRng);
                let nft_mint_key = nft_mint.pubkey();
                let signer: Arc<dyn Signer> = Arc::new(nft_mint);
                if !signing_keypairs.contains(&signer) {
                    signing_keypairs.push(signer);
                }

                let mut remaining_accounts = Vec::new();
                remaining_accounts.push(AccountMeta::new(tickarray_bitmap_extension, false));

                let open_position_instr = if traditional_nft {
                    clmm_instructions::open_position_instr(
                        &config.clone(),
                        pool_id,
                        result.vault0,
                        result.vault1,
                        result.mint0,
                        result.mint1,
                        nft_mint_key,
                        payer_pubkey,
                        deposit_token0,
                        deposit_token1,
                        remaining_accounts,
                        result.liquidity,
                        result.amount_0,
                        result.amount_1,
                        result.tick_lower_index,
                        result.tick_upper_index,
                        result.tick_array_lower_start_index,
                        result.tick_array_upper_start_index,
                        with_metadata,
                    )?
                } else {
                    clmm_instructions::open_position_with_token22_nft_instr(
                        &config.clone(),
                        pool_id,
                        result.vault0,
                        result.vault1,
                        result.mint0,
                        result.mint1,
                        nft_mint_key,
                        payer_pubkey,
                        deposit_token0,
                        deposit_token1,
                        remaining_accounts,
                        result.liquidity,
                        result.amount_0,
                        result.amount_1,
                        result.tick_lower_index,
                        result.tick_upper_index,
                        result.tick_array_lower_start_index,
                        result.tick_array_upper_start_index,
                        with_metadata,
                    )?
                };
                return Ok(Some(open_position_instr));
            } else {
                // personal position exist
                panic!("personal position exist:{:?}", find_position);
            }
        }
        ClmmCommands::IncreaseLiquidity {
            pool_id,
            deposit_token0,
            deposit_token1,
            tick_lower_price,
            tick_upper_price,
            amount_specified,
            base_token1,
        } => {
            let base_token0 = !base_token1;
            let result = clmm_utils::calculate_liquidity_change(
                &rpc_client,
                pool_id,
                tick_lower_price,
                tick_upper_price,
                amount_specified,
                config.slippage(),
                false,
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
            // load position
            let (_nft_tokens, positions) = clmm_utils::get_nft_accounts_and_positions_by_owner(
                &rpc_client,
                &payer_pubkey,
                &config.clmm_program(),
            );
            let rsps = rpc_client.get_multiple_accounts(&positions)?;
            let mut user_positions = Vec::new();
            for rsp in rsps {
                match rsp {
                    None => continue,
                    Some(rsp) => {
                        let position = common_utils::deserialize_anchor_account::<
                            raydium_amm_v3::states::PersonalPositionState,
                        >(&rsp)?;
                        user_positions.push(position);
                    }
                }
            }
            let mut find_position = raydium_amm_v3::states::PersonalPositionState::default();
            for position in user_positions {
                if position.pool_id == pool_id
                    && position.tick_lower_index == result.tick_lower_index
                    && position.tick_upper_index == result.tick_upper_index
                {
                    find_position = position.clone();
                }
            }
            if find_position.nft_mint != Pubkey::default() && find_position.pool_id == pool_id {
                // personal position exist
                let tickarray_bitmap_extension = Pubkey::find_program_address(
                    &[
                        raydium_amm_v3::states::POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(),
                        pool_id.to_bytes().as_ref(),
                    ],
                    &config.clmm_program(),
                )
                .0;
                let mut remaining_accounts = Vec::new();
                remaining_accounts.push(AccountMeta::new(tickarray_bitmap_extension, false));

                let increase_instr = clmm_instructions::increase_liquidity_instr(
                    &config.clone(),
                    pool_id,
                    result.vault0,
                    result.vault1,
                    result.mint0,
                    result.mint1,
                    find_position.nft_mint,
                    deposit_token0,
                    deposit_token1,
                    remaining_accounts,
                    result.liquidity,
                    result.amount_0,
                    result.amount_1,
                    result.tick_lower_index,
                    result.tick_upper_index,
                    result.tick_array_lower_start_index,
                    result.tick_array_upper_start_index,
                )?;
                return Ok(Some(increase_instr));
            } else {
                // personal position not exist
                panic!("personal position exist:{:?}", find_position);
            }
        }
        ClmmCommands::DecreaseLiquidity {
            pool_id,
            recipient_token0,
            recipient_token1,
            tick_lower_price,
            tick_upper_price,
            amount_specified,
            base_token1,
        } => {
            let base_token0 = !base_token1;
            let result = clmm_utils::calculate_liquidity_change(
                &rpc_client,
                pool_id,
                tick_lower_price,
                tick_upper_price,
                amount_specified,
                config.slippage(),
                true,
                base_token0,
            )?;
            // load position
            let (_nft_tokens, positions) = clmm_utils::get_nft_accounts_and_positions_by_owner(
                &rpc_client,
                &payer_pubkey,
                &config.clmm_program(),
            );
            let rsps = rpc_client.get_multiple_accounts(&positions)?;
            let mut user_positions = Vec::new();
            for rsp in rsps {
                match rsp {
                    None => continue,
                    Some(rsp) => {
                        let position = common_utils::deserialize_anchor_account::<
                            raydium_amm_v3::states::PersonalPositionState,
                        >(&rsp)?;
                        user_positions.push(position);
                    }
                }
            }
            let mut find_position = raydium_amm_v3::states::PersonalPositionState::default();
            for position in user_positions {
                if position.pool_id == pool_id
                    && position.tick_lower_index == result.tick_lower_index
                    && position.tick_upper_index == result.tick_upper_index
                {
                    find_position = position.clone();
                }
            }
            if find_position.nft_mint != Pubkey::default() && find_position.pool_id == pool_id {
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

                let mut reward_vault_with_user_token: Vec<Pubkey> = Vec::new();
                for item in result.reward_items.iter() {
                    // pool reward vault
                    reward_vault_with_user_token.push(item.reward_vault);
                    // user reward token
                    reward_vault_with_user_token.push(
                        spl_associated_token_account::get_associated_token_address_with_program_id(
                            &payer_pubkey,
                            &item.reward_mint,
                            &item.token_program,
                        ),
                    );
                    // reward vault mint
                    reward_vault_with_user_token.push(item.reward_mint);
                    // reward mint maybe token22
                    let create_user_reward_token_instr = token::create_ata_token_or_not(
                        &payer_pubkey,
                        &item.reward_mint,
                        &payer_pubkey,
                        Some(&item.token_program),
                    );
                    instructions.extend(create_user_reward_token_instr);
                }

                // personal position exist
                let tickarray_bitmap_extension = Pubkey::find_program_address(
                    &[
                        raydium_amm_v3::states::POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(),
                        pool_id.to_bytes().as_ref(),
                    ],
                    &config.clmm_program(),
                )
                .0;
                let mut remaining_accounts = Vec::new();
                remaining_accounts.push(AccountMeta::new(tickarray_bitmap_extension, false));
                // reward info
                let mut accounts = reward_vault_with_user_token
                    .into_iter()
                    .map(|item| AccountMeta::new(item, false))
                    .collect();
                remaining_accounts.append(&mut accounts);

                let decrease_instr = clmm_instructions::decrease_liquidity_instr(
                    &config.clone(),
                    pool_id,
                    result.vault0,
                    result.vault1,
                    result.mint0,
                    result.mint1,
                    find_position.nft_mint,
                    recipient_token0,
                    recipient_token1,
                    remaining_accounts,
                    result.liquidity,
                    result.amount_0,
                    result.amount_1,
                    result.tick_lower_index,
                    result.tick_upper_index,
                    result.tick_array_lower_start_index,
                    result.tick_array_upper_start_index,
                )?;
                instructions.extend(decrease_instr);
                return Ok(Some(instructions));
            } else {
                // personal position not exist
                panic!("personal position exist:{:?}", find_position);
            }
        }
        ClmmCommands::Swap {
            pool_id,
            user_input_token,
            user_output_token,
            amount_specified,
            limit_price,
            base_out,
        } => {
            let base_in = !base_out;
            let tickarray_bitmap_extension = Pubkey::find_program_address(
                &[
                    raydium_amm_v3::states::POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(),
                    pool_id.to_bytes().as_ref(),
                ],
                &config.clmm_program(),
            )
            .0;
            let result = clmm_utils::calculate_swap_change(
                &rpc_client,
                config.clmm_program(),
                pool_id,
                tickarray_bitmap_extension,
                user_input_token,
                amount_specified,
                limit_price,
                base_in,
                config.slippage(),
            )?;

            let mut instructions = Vec::new();
            let user_output_token = if let Some(user_output_token) = user_output_token {
                user_output_token
            } else {
                let create_user_output_token_instr = token::create_ata_token_or_not(
                    &payer_pubkey,
                    &result.output_vault_mint,
                    &payer_pubkey,
                    Some(&result.output_token_program),
                );
                instructions.extend(create_user_output_token_instr);

                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &result.output_vault_mint,
                    &result.output_token_program,
                )
            };

            let mut remaining_accounts = Vec::new();
            remaining_accounts.push(AccountMeta::new_readonly(tickarray_bitmap_extension, false));
            let mut accounts = result
                .remaining_tick_array_keys
                .into_iter()
                .map(|tick_array_address| AccountMeta::new(tick_array_address, false))
                .collect();
            remaining_accounts.append(&mut accounts);
            let swap_instr = clmm_instructions::swap_v2_instr(
                &config,
                result.pool_amm_config,
                result.pool_id,
                result.input_vault,
                result.output_vault,
                result.pool_observation,
                result.user_input_token,
                user_output_token,
                result.input_vault_mint,
                result.output_vault_mint,
                remaining_accounts,
                result.amount,
                result.other_amount_threshold,
                result.sqrt_price_limit_x64,
                result.is_base_input,
            )?;
            instructions.extend(swap_instr);
            return Ok(Some(instructions));
        }
        ClmmCommands::FetchPool {
            pool_id,
            mint0,
            mint1,
        } => {
            if let Some(pool_id) = pool_id {
                // fetch specified pool
                let pool_state = rpc::get_anchor_account::<raydium_amm_v3::states::PoolState>(
                    &rpc_client,
                    &pool_id,
                )
                .unwrap()
                .unwrap();
                println!("{:#?}", pool_state);
            } else {
                // fetch pools by filters
                let pool_len = raydium_amm_v3::states::PoolState::LEN as u64;
                let filters = match (mint0, mint1) {
                    (None, None) => Some(vec![RpcFilterType::DataSize(pool_len)]),
                    (Some(mint0), None) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 1 + 2 * 32,
                            &mint0.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                    (None, Some(mint1)) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 1 + 3 * 32,
                            &mint1.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                    (Some(mint0), Some(mint1)) => Some(vec![
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 1 + 2 * 32,
                            &mint0.to_bytes(),
                        )),
                        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                            8 + 1 + 3 * 32,
                            &mint1.to_bytes(),
                        )),
                        RpcFilterType::DataSize(pool_len),
                    ]),
                };
                let pools = rpc::get_program_accounts_with_filters(
                    &rpc_client,
                    config.clmm_program(),
                    filters,
                )
                .unwrap();
                for pool in pools {
                    println!("pool_id:{}", pool.0);
                    println!(
                        "{:#?}",
                        common_utils::deserialize_anchor_account::<raydium_amm_v3::states::PoolState>(
                            &pool.1
                        )
                    );
                }
            }
            return Ok(None);
        }
        ClmmCommands::FetchConfig { amm_config } => {
            let mut config_info = "".to_string();
            if let Some(amm_config) = amm_config {
                // fetch specified amm_config
                let amm_config_state =
                    rpc::get_anchor_account::<raydium_amm_v3::states::AmmConfig>(
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
                    "amm_config:{}, index:{}, tick_spacing:{}, trade: {:.2}%, protocol: {:.2}%, fund: {:.2}% \n",
                    amm_config,
                    amm_config_state.index,
                    amm_config_state.tick_spacing,
                    trade_fee_rate,
                    protocol_fee_rate,
                    fund_fee_rate
                );
                config_info.push_str(string.as_str());
            } else {
                // fetch all amm_config
                let amm_configs = rpc::get_program_accounts_with_filters(
                    &rpc_client,
                    config.clmm_program(),
                    Some(vec![RpcFilterType::DataSize(
                        raydium_amm_v3::states::AmmConfig::LEN as u64,
                    )]),
                )
                .unwrap();
                for amm_config in amm_configs {
                    let amm_config_state = common_utils::deserialize_anchor_account::<
                        raydium_amm_v3::states::AmmConfig,
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
                        "amm_config:{}, index:{}, tick_spacing:{}, trade: {:.2}%, protocol: {:.2}%, fund: {:.2}% \n",
                        amm_config.0,
                        amm_config_state.index,
                        amm_config_state.tick_spacing,
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
        ClmmCommands::DecodeIx { ix_data } => {
            decode_clmm_ix_event::handle_program_instruction(
                ix_data.as_str(),
                common_types::InstructionDecodeType::BaseHex,
            )?;
            return Ok(None);
        }
        ClmmCommands::DecodeEvent { event_data } => {
            decode_clmm_ix_event::handle_program_event(event_data.as_str(), false)?;
            return Ok(None);
        }
    }
}
