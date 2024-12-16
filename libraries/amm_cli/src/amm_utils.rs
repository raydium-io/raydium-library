use anyhow::Result;
use arrayref::array_ref;

use crate::{
    amm_math,
    amm_types::{AmmDepositInfoResult, AmmKeys, AmmSwapInfoResult, AmmWithdrawInfoResult},
};
use common::{common_utils, rpc};
use raydium_amm::state::Loadable;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

pub fn calculate_deposit_info(
    rpc_client: &RpcClient,
    amm_program: Pubkey,
    pool_id: Pubkey,
    amount_specified: u64,
    another_min_limit: bool,
    slippage_bps: u64,
    base_side: u64,
) -> Result<AmmDepositInfoResult> {
    // load amm keys
    let amm_keys = load_amm_keys(&rpc_client, &amm_program, &pool_id).unwrap();
    // reload accounts data to calculate amm pool vault amount
    // get multiple accounts at the same time to ensure data consistency
    let load_pubkeys = vec![
        pool_id,
        amm_keys.amm_target,
        amm_keys.amm_pc_vault,
        amm_keys.amm_coin_vault,
    ];
    let rsps = rpc::get_multiple_accounts(&rpc_client, &load_pubkeys).unwrap();
    let accounts = array_ref![rsps, 0, 4];
    let [amm_account, amm_target_account, amm_pc_vault_account, amm_coin_vault_account] = accounts;

    let amm_state =
        raydium_amm::state::AmmInfo::load_from_bytes(&amm_account.as_ref().unwrap().data).unwrap();
    let mut amm_state = amm_state.clone();
    let amm_target_state = raydium_amm::state::TargetOrders::load_from_bytes(
        &amm_target_account.as_ref().unwrap().data,
    )
    .unwrap();
    let amm_pc_vault =
        common_utils::unpack_token(&amm_pc_vault_account.as_ref().unwrap().data).unwrap();
    let amm_coin_vault =
        common_utils::unpack_token(&amm_coin_vault_account.as_ref().unwrap().data).unwrap();

    // assert for amm not share any liquidity to openbook
    assert_eq!(
        raydium_amm::state::AmmStatus::from_u64(amm_state.status).orderbook_permission(),
        false
    );
    // calculate pool vault amount without take pnl
    let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
        raydium_amm::math::Calculator::calc_total_without_take_pnl_no_orderbook(
            amm_pc_vault.base.amount,
            amm_coin_vault.base.amount,
            &amm_state,
        )
        .unwrap();
    // calculate pool vault amount after take pnl
    let (pool_pc_vault_amount, pool_coin_vault_amount) = amm_math::pool_vault_deduct_pnl(
        amm_pool_pc_vault_amount,
        amm_pool_coin_vault_amount,
        &mut amm_state,
        &amm_target_state,
    )
    .unwrap();

    let (max_coin_amount, max_pc_amount, another_min_amount) =
        amm_math::deposit_amount_with_slippage(
            pool_pc_vault_amount,
            pool_coin_vault_amount,
            amount_specified,
            another_min_limit,
            base_side,
            slippage_bps,
        )
        .unwrap();
    Ok(AmmDepositInfoResult {
        pool_id,
        amm_authority: amm_keys.amm_authority,
        amm_open_orders: amm_keys.amm_open_order,
        amm_target_orders: amm_keys.amm_target,
        amm_lp_mint: amm_keys.amm_lp_mint,
        amm_coin_mint: amm_keys.amm_coin_mint,
        amm_pc_mint: amm_keys.amm_pc_mint,
        amm_coin_vault: amm_keys.amm_coin_vault,
        amm_pc_vault: amm_keys.amm_pc_vault,
        market: amm_keys.amm_open_order, // padding readonly account
        market_event_queue: amm_keys.amm_open_order, // padding readonly account
        max_coin_amount,
        max_pc_amount,
        another_min_amount,
        base_side,
    })
}

pub fn calculate_withdraw_info(
    rpc_client: &RpcClient,
    amm_program: Pubkey,
    pool_id: Pubkey,
    input_lp_amount: u64,
    slippage_bps: Option<u64>,
) -> Result<AmmWithdrawInfoResult> {
    // load amm keys
    let amm_keys = load_amm_keys(&rpc_client, &amm_program, &pool_id).unwrap();
    // reload accounts data to calculate amm pool vault amount
    // get multiple accounts at the same time to ensure data consistency
    let load_pubkeys = vec![
        pool_id,
        amm_keys.amm_target,
        amm_keys.amm_pc_vault,
        amm_keys.amm_coin_vault,
    ];
    let rsps = rpc::get_multiple_accounts(&rpc_client, &load_pubkeys).unwrap();
    let accounts = array_ref![rsps, 0, 4];
    let [amm_account, amm_target_account, amm_pc_vault_account, amm_coin_vault_account] = accounts;

    let amm_state =
        raydium_amm::state::AmmInfo::load_from_bytes(&amm_account.as_ref().unwrap().data).unwrap();
    let mut amm_state = amm_state.clone();
    let amm_target_state = raydium_amm::state::TargetOrders::load_from_bytes(
        &amm_target_account.as_ref().unwrap().data,
    )
    .unwrap();
    let amm_pc_vault =
        common_utils::unpack_token(&amm_pc_vault_account.as_ref().unwrap().data).unwrap();
    let amm_coin_vault =
        common_utils::unpack_token(&amm_coin_vault_account.as_ref().unwrap().data).unwrap();

    // assert for amm not share any liquidity to openbook
    assert_eq!(
        raydium_amm::state::AmmStatus::from_u64(amm_state.status).orderbook_permission(),
        false
    );
    // calculate pool vault amount without take pnl
    let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
        raydium_amm::math::Calculator::calc_total_without_take_pnl_no_orderbook(
            amm_pc_vault.base.amount,
            amm_coin_vault.base.amount,
            &amm_state,
        )
        .unwrap();
    // calculate pool vault amount after take pnl
    let (pool_pc_vault_amount, pool_coin_vault_amount) = amm_math::pool_vault_deduct_pnl(
        amm_pool_pc_vault_amount,
        amm_pool_coin_vault_amount,
        &mut amm_state,
        &amm_target_state,
    )
    .unwrap();

    let (receive_min_coin_amount, receive_min_pc_amount) =
        amm_math::withdraw_amounts_with_slippage(
            pool_pc_vault_amount,
            pool_coin_vault_amount,
            amm_state.lp_amount,
            input_lp_amount,
            slippage_bps,
        )
        .unwrap();
    Ok(AmmWithdrawInfoResult {
        pool_id,
        amm_authority: amm_keys.amm_authority,
        amm_open_orders: amm_keys.amm_open_order,
        amm_target_orders: amm_keys.amm_target,
        amm_lp_mint: amm_keys.amm_lp_mint,
        amm_coin_vault: amm_keys.amm_coin_vault,
        amm_pc_vault: amm_keys.amm_pc_vault,
        amm_coin_mint: amm_keys.amm_coin_mint,
        amm_pc_mint: amm_keys.amm_pc_mint,
        market_program: amm_keys.amm_authority, // padding readonly account
        market: amm_keys.amm_open_order,        // padding readwrite account
        market_coin_vault: amm_keys.amm_open_order, //padding readwrite account
        market_pc_vault: amm_keys.amm_open_order, //padding readwrite account
        market_vault_signer: amm_keys.amm_authority, // padding readonly account
        market_event_queue: amm_keys.amm_open_order, // padding readwrite account
        market_bids: amm_keys.amm_open_order,   // padding readwrite account
        market_asks: amm_keys.amm_open_order,   // padding readwrite account
        receive_min_coin_amount,
        receive_min_pc_amount,
    })
}

pub fn calculate_swap_info(
    rpc_client: &RpcClient,
    amm_program: Pubkey,
    pool_id: Pubkey,
    user_input_token: Pubkey,
    amount_specified: u64,
    slippage_bps: u64,
    base_in: bool,
) -> Result<AmmSwapInfoResult> {
    // load amm keys
    let amm_keys = load_amm_keys(&rpc_client, &amm_program, &pool_id).unwrap();
    // reload accounts data to calculate amm pool vault amount
    // get multiple accounts at the same time to ensure data consistency
    let load_pubkeys = vec![
        pool_id,
        amm_keys.amm_pc_vault,
        amm_keys.amm_coin_vault,
        user_input_token,
    ];
    let rsps = rpc::get_multiple_accounts(&rpc_client, &load_pubkeys).unwrap();
    let accounts = array_ref![rsps, 0, 4];
    let [amm_account, amm_pc_vault_account, amm_coin_vault_account, user_input_token_account] =
        accounts;

    let amm_state =
        raydium_amm::state::AmmInfo::load_from_bytes(&amm_account.as_ref().unwrap().data).unwrap();
    let amm_state = amm_state.clone();
    let amm_pc_vault =
        common_utils::unpack_token(&amm_pc_vault_account.as_ref().unwrap().data).unwrap();
    let amm_coin_vault =
        common_utils::unpack_token(&amm_coin_vault_account.as_ref().unwrap().data).unwrap();
    let user_input_token_info =
        common_utils::unpack_token(&user_input_token_account.as_ref().unwrap().data).unwrap();

    // assert for amm not share any liquidity to openbook
    assert_eq!(
        raydium_amm::state::AmmStatus::from_u64(amm_state.status).orderbook_permission(),
        false
    );
    // calculate pool vault amount without take pnl
    let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
        raydium_amm::math::Calculator::calc_total_without_take_pnl_no_orderbook(
            amm_pc_vault.base.amount,
            amm_coin_vault.base.amount,
            &amm_state,
        )
        .unwrap();

    let (swap_direction, input_mint, output_mint) =
        if user_input_token_info.base.mint == amm_keys.amm_coin_mint {
            (
                raydium_amm::math::SwapDirection::Coin2PC,
                amm_keys.amm_coin_mint,
                amm_keys.amm_pc_mint,
            )
        } else if user_input_token_info.base.mint == amm_keys.amm_pc_mint {
            (
                raydium_amm::math::SwapDirection::PC2Coin,
                amm_keys.amm_pc_mint,
                amm_keys.amm_coin_mint,
            )
        } else {
            panic!("input tokens not match pool vaults");
        };
    let other_amount_threshold = amm_math::swap_with_slippage(
        amm_pool_pc_vault_amount,
        amm_pool_coin_vault_amount,
        amm_state.fees.swap_fee_numerator,
        amm_state.fees.swap_fee_denominator,
        swap_direction,
        amount_specified,
        base_in,
        slippage_bps,
    )?;

    Ok(AmmSwapInfoResult {
        pool_id,
        amm_authority: amm_keys.amm_authority,
        amm_open_orders: amm_keys.amm_open_order,
        amm_coin_vault: amm_keys.amm_coin_vault,
        amm_pc_vault: amm_keys.amm_pc_vault,
        input_mint,
        output_mint,
        market_program: amm_keys.amm_authority, // padding readonly account
        market: amm_keys.amm_open_order,        // padding readwrite account
        market_coin_vault: amm_keys.amm_open_order, // padding readwrite account
        market_pc_vault: amm_keys.amm_open_order, // padding readwrite account
        market_vault_signer: amm_keys.amm_authority, // padding readonly account
        market_event_queue: amm_keys.amm_open_order, // padding readwrite account
        market_bids: amm_keys.amm_open_order,   // padding readwrite account
        market_asks: amm_keys.amm_open_order,   // padding readwrite account
        amount_specified,
        other_amount_threshold,
    })
}

// only use for initialize_amm_pool, because the keys of some amm pools are not used in this way.
pub fn get_amm_pda_keys(
    amm_program: &Pubkey,
    market_program: &Pubkey,
    market: &Pubkey,
    coin_mint: &Pubkey,
    pc_mint: &Pubkey,
) -> Result<AmmKeys> {
    let amm_pool = raydium_amm::processor::get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        raydium_amm::processor::AMM_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let (amm_authority, nonce) =
        Pubkey::find_program_address(&[raydium_amm::processor::AUTHORITY_AMM], &amm_program);
    let amm_open_order = raydium_amm::processor::get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        raydium_amm::processor::OPEN_ORDER_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_lp_mint = raydium_amm::processor::get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        raydium_amm::processor::LP_MINT_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_coin_vault = raydium_amm::processor::get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        raydium_amm::processor::COIN_VAULT_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_pc_vault = raydium_amm::processor::get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        raydium_amm::processor::PC_VAULT_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_target = raydium_amm::processor::get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        raydium_amm::processor::TARGET_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;

    Ok(AmmKeys {
        amm_pool,
        amm_target,
        amm_coin_vault,
        amm_pc_vault,
        amm_lp_mint,
        amm_open_order,
        amm_coin_mint: *coin_mint,
        amm_pc_mint: *pc_mint,
        amm_authority,
        market: *market,
        market_program: *market_program,
        nonce,
    })
}

pub fn load_amm_keys(
    client: &RpcClient,
    amm_program: &Pubkey,
    amm_pool: &Pubkey,
) -> Result<AmmKeys> {
    let amm_data = rpc::get_account(client, &amm_pool)?.unwrap();
    let amm = raydium_amm::state::AmmInfo::load_from_bytes(&amm_data).unwrap();
    Ok(AmmKeys {
        amm_pool: *amm_pool,
        amm_target: amm.target_orders,
        amm_coin_vault: amm.coin_vault,
        amm_pc_vault: amm.pc_vault,
        amm_lp_mint: amm.lp_mint,
        amm_open_order: amm.open_orders,
        amm_coin_mint: amm.coin_vault_mint,
        amm_pc_mint: amm.pc_vault_mint,
        amm_authority: raydium_amm::processor::Processor::authority_id(
            amm_program,
            raydium_amm::processor::AUTHORITY_AMM,
            amm.nonce as u8,
        )?,
        market: amm.market,
        market_program: amm.market_program,
        nonce: amm.nonce as u8,
    })
}
