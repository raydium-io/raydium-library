use anchor_client::Client;
use anchor_lang::prelude::AccountMeta;
use anchor_spl::{memo::ID as MEMO_ID, metadata::mpl_token_metadata::ID as MPL_ID};
use anyhow::{format_err, Result};
use common::common_types::CommonConfig;
use raydium_amm_v3::{
    accounts as raydium_clmm_accounts, instruction as raydium_clmm_instruction,
    states::{
        OBSERVATION_SEED, POOL_SEED, POOL_TICK_ARRAY_BITMAP_SEED, POOL_VAULT_SEED, POSITION_SEED,
        TICK_ARRAY_SEED,
    },
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_program, sysvar};
use std::rc::Rc;
const MPL_PREFIX: &str = "metadata";

pub fn create_pool_instr(
    config: &CommonConfig,
    amm_config: Pubkey,
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    token_program_0: Pubkey,
    token_program_1: Pubkey,
    sqrt_price_x64: u128,
    open_time: u64,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let (pool_account_key, __bump) = Pubkey::find_program_address(
        &[
            POOL_SEED.as_bytes(),
            amm_config.to_bytes().as_ref(),
            token_mint_0.to_bytes().as_ref(),
            token_mint_1.to_bytes().as_ref(),
        ],
        &program.id(),
    );
    let (token_vault_0, __bump) = Pubkey::find_program_address(
        &[
            POOL_VAULT_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            token_mint_0.to_bytes().as_ref(),
        ],
        &program.id(),
    );
    let (token_vault_1, __bump) = Pubkey::find_program_address(
        &[
            POOL_VAULT_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            token_mint_1.to_bytes().as_ref(),
        ],
        &program.id(),
    );
    let (tick_array_bitmap, __bump) = Pubkey::find_program_address(
        &[
            POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
        ],
        &program.id(),
    );
    let (observation_key, __bump) = Pubkey::find_program_address(
        &[
            OBSERVATION_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
        ],
        &program.id(),
    );
    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::CreatePool {
            pool_creator: program.payer(),
            amm_config,
            pool_state: pool_account_key,
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            observation_state: observation_key,
            tick_array_bitmap,
            token_program_0,
            token_program_1,
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
        })
        .args(raydium_clmm_instruction::CreatePool {
            sqrt_price_x64,
            open_time,
        })
        .instructions()?;
    Ok(instructions)
}

pub fn open_position_instr(
    config: &CommonConfig,
    pool_account_key: Pubkey,
    token_vault_0: Pubkey,
    token_vault_1: Pubkey,
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    nft_mint_key: Pubkey,
    nft_to_owner: Pubkey,
    user_token_account_0: Pubkey,
    user_token_account_1: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    liquidity: u128,
    amount_0_max: u64,
    amount_1_max: u64,
    tick_lower_index: i32,
    tick_upper_index: i32,
    tick_array_lower_start_index: i32,
    tick_array_upper_start_index: i32,
    with_metadata: bool,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let nft_ata_token_account =
        spl_associated_token_account::get_associated_token_address(&program.payer(), &nft_mint_key);
    let (metadata_account_key, _bump) = Pubkey::find_program_address(
        &[
            MPL_PREFIX.as_bytes(),
            MPL_ID.to_bytes().as_ref(),
            nft_mint_key.to_bytes().as_ref(),
        ],
        &MPL_ID,
    );
    let (protocol_position_key, __bump) = Pubkey::find_program_address(
        &[
            POSITION_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_lower_index.to_be_bytes(),
            &tick_upper_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (tick_array_lower, __bump) = Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_array_lower_start_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (tick_array_upper, __bump) = Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_array_upper_start_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (personal_position_key, __bump) = Pubkey::find_program_address(
        &[POSITION_SEED.as_bytes(), nft_mint_key.to_bytes().as_ref()],
        &program.id(),
    );
    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::OpenPositionV2 {
            payer: program.payer(),
            position_nft_owner: nft_to_owner,
            position_nft_mint: nft_mint_key,
            position_nft_account: nft_ata_token_account,
            metadata_account: metadata_account_key,
            pool_state: pool_account_key,
            protocol_position: protocol_position_key,
            tick_array_lower,
            tick_array_upper,
            personal_position: personal_position_key,
            token_account_0: user_token_account_0,
            token_account_1: user_token_account_1,
            token_vault_0,
            token_vault_1,
            rent: sysvar::rent::id(),
            system_program: system_program::id(),
            token_program: spl_token::id(),
            associated_token_program: spl_associated_token_account::id(),
            metadata_program: MPL_ID,
            token_program_2022: spl_token_2022::id(),
            vault_0_mint: token_mint_0,
            vault_1_mint: token_mint_1,
        })
        .accounts(remaining_accounts)
        .args(raydium_clmm_instruction::OpenPositionV2 {
            liquidity,
            amount_0_max,
            amount_1_max,
            tick_lower_index,
            tick_upper_index,
            tick_array_lower_start_index,
            tick_array_upper_start_index,
            with_metadata,
            base_flag: None,
        })
        .instructions()?;
    Ok(instructions)
}

pub fn increase_liquidity_instr(
    config: &CommonConfig,
    pool_account_key: Pubkey,
    token_vault_0: Pubkey,
    token_vault_1: Pubkey,
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    nft_mint_key: Pubkey,
    user_token_account_0: Pubkey,
    user_token_account_1: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    liquidity: u128,
    amount_0_max: u64,
    amount_1_max: u64,
    tick_lower_index: i32,
    tick_upper_index: i32,
    tick_array_lower_start_index: i32,
    tick_array_upper_start_index: i32,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let nft_ata_token_account =
        spl_associated_token_account::get_associated_token_address(&program.payer(), &nft_mint_key);
    let (tick_array_lower, __bump) = Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_array_lower_start_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (tick_array_upper, __bump) = Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_array_upper_start_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (protocol_position_key, __bump) = Pubkey::find_program_address(
        &[
            POSITION_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_lower_index.to_be_bytes(),
            &tick_upper_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (personal_position_key, __bump) = Pubkey::find_program_address(
        &[POSITION_SEED.as_bytes(), nft_mint_key.to_bytes().as_ref()],
        &program.id(),
    );

    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::IncreaseLiquidityV2 {
            nft_owner: program.payer(),
            nft_account: nft_ata_token_account,
            pool_state: pool_account_key,
            protocol_position: protocol_position_key,
            personal_position: personal_position_key,
            tick_array_lower,
            tick_array_upper,
            token_account_0: user_token_account_0,
            token_account_1: user_token_account_1,
            token_vault_0,
            token_vault_1,
            token_program: spl_token::id(),
            token_program_2022: spl_token_2022::id(),
            vault_0_mint: token_mint_0,
            vault_1_mint: token_mint_1,
        })
        .accounts(remaining_accounts)
        .args(raydium_clmm_instruction::IncreaseLiquidityV2 {
            liquidity,
            amount_0_max,
            amount_1_max,
            base_flag: None,
        })
        .instructions()?;
    Ok(instructions)
}

pub fn decrease_liquidity_instr(
    config: &CommonConfig,
    pool_account_key: Pubkey,
    token_vault_0: Pubkey,
    token_vault_1: Pubkey,
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    nft_mint_key: Pubkey,
    user_token_account_0: Pubkey,
    user_token_account_1: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    liquidity: u128,
    amount_0_min: u64,
    amount_1_min: u64,
    tick_lower_index: i32,
    tick_upper_index: i32,
    tick_array_lower_start_index: i32,
    tick_array_upper_start_index: i32,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let nft_ata_token_account =
        spl_associated_token_account::get_associated_token_address(&program.payer(), &nft_mint_key);
    let (personal_position_key, __bump) = Pubkey::find_program_address(
        &[POSITION_SEED.as_bytes(), nft_mint_key.to_bytes().as_ref()],
        &program.id(),
    );
    let (protocol_position_key, __bump) = Pubkey::find_program_address(
        &[
            POSITION_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_lower_index.to_be_bytes(),
            &tick_upper_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (tick_array_lower, __bump) = Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_array_lower_start_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let (tick_array_upper, __bump) = Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED.as_bytes(),
            pool_account_key.to_bytes().as_ref(),
            &tick_array_upper_start_index.to_be_bytes(),
        ],
        &program.id(),
    );
    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::DecreaseLiquidityV2 {
            nft_owner: program.payer(),
            nft_account: nft_ata_token_account,
            personal_position: personal_position_key,
            pool_state: pool_account_key,
            protocol_position: protocol_position_key,
            token_vault_0,
            token_vault_1,
            tick_array_lower,
            tick_array_upper,
            recipient_token_account_0: user_token_account_0,
            recipient_token_account_1: user_token_account_1,
            token_program: spl_token::id(),
            token_program_2022: spl_token_2022::id(),
            memo_program: MEMO_ID,
            vault_0_mint: token_mint_0,
            vault_1_mint: token_mint_1,
        })
        .accounts(remaining_accounts)
        .args(raydium_clmm_instruction::DecreaseLiquidityV2 {
            liquidity,
            amount_0_min,
            amount_1_min,
        })
        .instructions()?;
    Ok(instructions)
}

pub fn close_personal_position_instr(
    config: &CommonConfig,
    nft_mint_key: Pubkey,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let nft_ata_token_account =
        spl_associated_token_account::get_associated_token_address(&program.payer(), &nft_mint_key);
    let (personal_position_key, __bump) = Pubkey::find_program_address(
        &[POSITION_SEED.as_bytes(), nft_mint_key.to_bytes().as_ref()],
        &program.id(),
    );
    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::ClosePosition {
            nft_owner: program.payer(),
            position_nft_mint: nft_mint_key,
            position_nft_account: nft_ata_token_account,
            personal_position: personal_position_key,
            system_program: system_program::id(),
            token_program: spl_token::id(),
        })
        .args(raydium_clmm_instruction::ClosePosition)
        .instructions()?;
    Ok(instructions)
}

pub fn swap_instr(
    config: &CommonConfig,
    amm_config: Pubkey,
    pool_account_key: Pubkey,
    input_vault: Pubkey,
    output_vault: Pubkey,
    observation_state: Pubkey,
    user_input_token: Pubkey,
    user_out_put_token: Pubkey,
    tick_array: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit_x64: Option<u128>,
    is_base_input: bool,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::SwapSingle {
            payer: program.payer(),
            amm_config,
            pool_state: pool_account_key,
            input_token_account: user_input_token,
            output_token_account: user_out_put_token,
            input_vault,
            output_vault,
            tick_array,
            observation_state,
            token_program: spl_token::id(),
        })
        .accounts(remaining_accounts)
        .args(raydium_clmm_instruction::Swap {
            amount,
            other_amount_threshold,
            sqrt_price_limit_x64: sqrt_price_limit_x64.unwrap_or(0u128),
            is_base_input,
        })
        .instructions()?;
    Ok(instructions)
}

pub fn swap_v2_instr(
    config: &CommonConfig,
    amm_config: Pubkey,
    pool_account_key: Pubkey,
    input_vault: Pubkey,
    output_vault: Pubkey,
    observation_state: Pubkey,
    user_input_token: Pubkey,
    user_output_token: Pubkey,
    input_vault_mint: Pubkey,
    output_vault_mint: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit_x64: Option<u128>,
    is_base_input: bool,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::SwapSingleV2 {
            payer: program.payer(),
            amm_config,
            pool_state: pool_account_key,
            input_token_account: user_input_token,
            output_token_account: user_output_token,
            input_vault,
            output_vault,
            observation_state,
            token_program: spl_token::id(),
            token_program_2022: spl_token_2022::id(),
            memo_program: MEMO_ID,
            input_vault_mint,
            output_vault_mint,
        })
        .accounts(remaining_accounts)
        .args(raydium_clmm_instruction::SwapV2 {
            amount,
            other_amount_threshold,
            sqrt_price_limit_x64: sqrt_price_limit_x64.unwrap_or(0u128),
            is_base_input,
        })
        .instructions()?;
    Ok(instructions)
}

pub fn initialize_reward_instr(
    config: &CommonConfig,
    pool_account_key: Pubkey,
    amm_config: Pubkey,
    operation_account_key: Pubkey,
    reward_token_mint: Pubkey,
    reward_token_vault: Pubkey,
    user_reward_token: Pubkey,
    reward_token_program: Pubkey,
    open_time: u64,
    end_time: u64,
    emissions_per_second_x64: u128,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::InitializeReward {
            reward_funder: program.payer(),
            funder_token_account: user_reward_token,
            amm_config,
            pool_state: pool_account_key,
            operation_state: operation_account_key,
            reward_token_mint,
            reward_token_vault,
            reward_token_program,
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
        })
        .args(raydium_clmm_instruction::InitializeReward {
            param: raydium_amm_v3::instructions::InitializeRewardParam {
                open_time,
                end_time,
                emissions_per_second_x64,
            },
        })
        .instructions()?;
    Ok(instructions)
}

pub fn set_reward_params_instr(
    config: &CommonConfig,
    amm_config: Pubkey,
    pool_account_key: Pubkey,
    reward_token_vault: Pubkey,
    user_reward_token: Pubkey,
    operation_account_key: Pubkey,
    reward_index: u8,
    open_time: u64,
    end_time: u64,
    emissions_per_second_x64: u128,
) -> Result<Vec<Instruction>> {
    let wallet = solana_sdk::signature::read_keypair_file(config.wallet())
        .map_err(|_| format_err!("failed to read keypair from {}", config.wallet()))?;
    let cluster = config.cluster();
    // Client.
    let client = Client::new(cluster, Rc::new(wallet));
    let program = client.program(config.clmm_program())?;

    let remaining_accounts = vec![
        AccountMeta::new(reward_token_vault, false),
        AccountMeta::new(user_reward_token, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    let instructions = program
        .request()
        .accounts(raydium_clmm_accounts::SetRewardParams {
            authority: program.payer(),
            amm_config,
            pool_state: pool_account_key,
            operation_state: operation_account_key,
            token_program: spl_token::id(),
            token_program_2022: spl_token_2022::id(),
        })
        .accounts(remaining_accounts)
        .args(raydium_clmm_instruction::SetRewardParams {
            reward_index,
            emissions_per_second_x64,
            open_time,
            end_time,
        })
        .instructions()?;
    Ok(instructions)
}
