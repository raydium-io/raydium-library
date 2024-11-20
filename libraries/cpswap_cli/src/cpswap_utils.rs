use crate::cpswap_types::{CpSwapLiquidityChangeResult, CpSwapSwapChangeResult};
use anyhow::Result;
use arrayref::array_ref;
use common::{common_utils, rpc};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::convert::{TryFrom, TryInto};

pub fn specified_tokens_to_lp_tokens(
    amount_specified: u128,
    lp_token_supply: u128,
    swap_token_0_amount: u128,
    swap_token_1_amount: u128,
    base_token0: bool,
) -> u128 {
    let (amount0, amount1) = if base_token0 {
        let another_amount = amount_specified
            .checked_mul(swap_token_1_amount)
            .unwrap()
            .checked_div(swap_token_0_amount)
            .unwrap();
        (amount_specified, another_amount)
    } else {
        let another_amount = amount_specified
            .checked_mul(swap_token_0_amount)
            .unwrap()
            .checked_div(swap_token_1_amount)
            .unwrap();
        (another_amount, amount_specified)
    };
    let liquidity = std::cmp::min(
        amount0
            .checked_mul(lp_token_supply)
            .unwrap()
            .checked_div(swap_token_0_amount)
            .unwrap(),
        amount1
            .checked_mul(lp_token_supply)
            .unwrap()
            .checked_div(swap_token_1_amount)
            .unwrap(),
    );
    liquidity
}

pub fn add_liquidity_calculate(
    rpc_client: &RpcClient,
    pool_id: Pubkey,
    amount_specified: u64,
    slippage_bps: u64,
    base_token0: bool,
) -> Result<CpSwapLiquidityChangeResult> {
    let pool_state =
        rpc::get_anchor_account::<raydium_cp_swap::states::PoolState>(rpc_client, &pool_id)
            .unwrap()
            .unwrap();
    // load account
    let load_pubkeys = vec![
        pool_state.token_0_vault,
        pool_state.token_1_vault,
        pool_state.token_0_mint,
        pool_state.token_1_mint,
    ];
    let rsps = rpc_client.get_multiple_accounts(&load_pubkeys).unwrap();
    let [token_0_vault_account, token_1_vault_account, token_0_mint_account, token_1_mint_account] =
        array_ref![rsps, 0, 4];
    // docode account
    let token_0_vault_info =
        common_utils::unpack_token(&token_0_vault_account.as_ref().unwrap().data).unwrap();
    let token_1_vault_info =
        common_utils::unpack_token(&token_1_vault_account.as_ref().unwrap().data).unwrap();
    let token_0_mint_info =
        common_utils::unpack_mint(&token_0_mint_account.as_ref().unwrap().data).unwrap();
    let token_1_mint_info =
        common_utils::unpack_mint(&token_1_mint_account.as_ref().unwrap().data).unwrap();
    let epoch = rpc_client.get_epoch_info().unwrap().epoch;

    let (total_token_0_amount, total_token_1_amount) = pool_state.vault_amount_without_fee(
        token_0_vault_info.base.amount,
        token_1_vault_info.base.amount,
    );

    // calculate amount_specified without transfer fee
    let transfer_fee = if base_token0 {
        common_utils::get_transfer_fee(&token_0_mint_info, epoch, amount_specified)
    } else {
        common_utils::get_transfer_fee(&token_1_mint_info, epoch, amount_specified)
    };
    let specified_without_fee = amount_specified.checked_sub(transfer_fee).unwrap();
    // calculate lp_amount by amount_specified
    let liquidity = specified_tokens_to_lp_tokens(
        specified_without_fee.into(),
        pool_state.lp_supply.into(),
        total_token_0_amount.into(),
        total_token_1_amount.into(),
        base_token0,
    );
    // calculate amounts by liquidity
    let results = raydium_cp_swap::curve::CurveCalculator::lp_tokens_to_trading_tokens(
        liquidity,
        u128::from(pool_state.lp_supply),
        u128::from(total_token_0_amount),
        u128::from(total_token_1_amount),
        raydium_cp_swap::curve::RoundDirection::Ceiling,
    )
    .ok_or(raydium_cp_swap::error::ErrorCode::ZeroTradingTokens)
    .unwrap();
    println!(
        "amount_0:{}, amount_1:{}, lp_token_amount:{}",
        results.token_0_amount, results.token_1_amount, liquidity
    );
    // calculate another amount with transfer fee
    let another_amount = if base_token0 {
        let token_1_amount: u64 = results.token_1_amount.try_into().unwrap();
        let transfer_fee =
            common_utils::get_transfer_inverse_fee(&token_1_mint_info, epoch, token_1_amount);
        token_1_amount.checked_add(transfer_fee).unwrap()
    } else {
        let token_0_amount = results.token_0_amount.try_into().unwrap();
        let transfer_fee =
            common_utils::get_transfer_inverse_fee(&token_0_mint_info, epoch, token_0_amount);
        token_0_amount.checked_add(transfer_fee).unwrap()
    };
    // calc liquidity with slippage
    let liquidity_slippage =
        common_utils::amount_with_slippage(liquidity as u64, slippage_bps, false)?;

    let (amount_0_max, amount_1_max) = if base_token0 {
        (amount_specified, another_amount)
    } else {
        (another_amount, amount_specified)
    };

    Ok(CpSwapLiquidityChangeResult {
        pool_id,
        mint0: pool_state.token_0_mint,
        mint1: pool_state.token_1_mint,
        mintlp: pool_state.lp_mint,
        vault0: pool_state.token_0_vault,
        vault1: pool_state.token_1_vault,
        mint0_token_program: pool_state.token_0_program,
        mint1_token_program: pool_state.token_1_program,
        lp_token_amount: liquidity_slippage,
        amount_0: amount_0_max,
        amount_1: amount_1_max,
    })
}

pub fn remove_liquidity_calculate(
    rpc_client: &RpcClient,
    pool_id: Pubkey,
    input_lp_amount: u64,
    slippage_bps: u64,
) -> Result<CpSwapLiquidityChangeResult> {
    let pool_state =
        rpc::get_anchor_account::<raydium_cp_swap::states::PoolState>(rpc_client, &pool_id)
            .unwrap()
            .unwrap();
    // load account
    let load_pubkeys = vec![
        pool_state.token_0_vault,
        pool_state.token_1_vault,
        pool_state.token_0_mint,
        pool_state.token_1_mint,
    ];
    let rsps = rpc_client.get_multiple_accounts(&load_pubkeys).unwrap();
    let [token_0_vault_account, token_1_vault_account, token_0_mint_account, token_1_mint_account] =
        array_ref![rsps, 0, 4];
    // docode account
    let token_0_vault_info =
        common_utils::unpack_token(&token_0_vault_account.as_ref().unwrap().data).unwrap();
    let token_1_vault_info =
        common_utils::unpack_token(&token_1_vault_account.as_ref().unwrap().data).unwrap();
    let token_0_mint_info =
        common_utils::unpack_mint(&token_0_mint_account.as_ref().unwrap().data).unwrap();
    let token_1_mint_info =
        common_utils::unpack_mint(&token_1_mint_account.as_ref().unwrap().data).unwrap();
    let epoch = rpc_client.get_epoch_info().unwrap().epoch;

    let (total_token_0_amount, total_token_1_amount) = pool_state.vault_amount_without_fee(
        token_0_vault_info.base.amount,
        token_1_vault_info.base.amount,
    );
    // calculate amount
    let results = raydium_cp_swap::curve::CurveCalculator::lp_tokens_to_trading_tokens(
        u128::from(input_lp_amount),
        u128::from(pool_state.lp_supply),
        u128::from(total_token_0_amount),
        u128::from(total_token_1_amount),
        raydium_cp_swap::curve::RoundDirection::Floor,
    )
    .ok_or(raydium_cp_swap::error::ErrorCode::ZeroTradingTokens)
    .unwrap();
    println!(
        "amount_0:{}, amount_1:{}, input_lp_amount:{}",
        results.token_0_amount, results.token_1_amount, input_lp_amount
    );
    // calc with slippage
    let amount_0_with_slippage =
        common_utils::amount_with_slippage(results.token_0_amount as u64, slippage_bps, false)?;
    let amount_1_with_slippage =
        common_utils::amount_with_slippage(results.token_1_amount as u64, slippage_bps, false)?;
    // calc with transfer_fee
    let transfer_fee_0 =
        common_utils::get_transfer_inverse_fee(&token_0_mint_info, epoch, amount_0_with_slippage);
    let transfer_fee_1 =
        common_utils::get_transfer_inverse_fee(&token_1_mint_info, epoch, amount_1_with_slippage);
    println!(
        "transfer_fee_0:{}, transfer_fee_1:{}",
        transfer_fee_0, transfer_fee_1
    );
    let amount_0_max = amount_0_with_slippage.checked_add(transfer_fee_0).unwrap();
    let amount_1_max = amount_1_with_slippage.checked_add(transfer_fee_1).unwrap();
    println!(
        "amount_0_max:{}, amount_1_max:{}",
        amount_0_max, amount_1_max
    );
    Ok(CpSwapLiquidityChangeResult {
        pool_id,
        mint0: pool_state.token_0_mint,
        mint1: pool_state.token_1_mint,
        mintlp: pool_state.lp_mint,
        vault0: pool_state.token_0_vault,
        vault1: pool_state.token_1_vault,
        mint0_token_program: pool_state.token_0_program,
        mint1_token_program: pool_state.token_1_program,
        lp_token_amount: input_lp_amount,
        amount_0: amount_0_max,
        amount_1: amount_1_max,
    })
}

pub fn swap_calculate(
    rpc_client: &RpcClient,
    pool_id: Pubkey,
    user_input_token: Pubkey,
    amount_specified: u64,
    slippage_bps: u64,
    base_in: bool,
) -> Result<CpSwapSwapChangeResult> {
    let pool_state =
        rpc::get_anchor_account::<raydium_cp_swap::states::PoolState>(&rpc_client, &pool_id)
            .unwrap()
            .unwrap();

    // load account
    let load_pubkeys = vec![
        pool_state.amm_config,
        pool_state.token_0_vault,
        pool_state.token_1_vault,
        pool_state.token_0_mint,
        pool_state.token_1_mint,
        user_input_token,
    ];
    let rsps = rpc_client.get_multiple_accounts(&load_pubkeys).unwrap();
    let epoch = rpc_client.get_epoch_info().unwrap().epoch;
    let [amm_config_account, token_0_vault_account, token_1_vault_account, token_0_mint_account, token_1_mint_account, user_input_token_account] =
        array_ref![rsps, 0, 6];
    // docode account
    let amm_config_state = common_utils::deserialize_anchor_account::<
        raydium_cp_swap::states::AmmConfig,
    >(amm_config_account.as_ref().unwrap())
    .unwrap();

    let token_0_vault_info =
        common_utils::unpack_token(&token_0_vault_account.as_ref().unwrap().data).unwrap();
    let token_1_vault_info =
        common_utils::unpack_token(&token_1_vault_account.as_ref().unwrap().data).unwrap();
    let token_0_mint_info =
        common_utils::unpack_mint(&token_0_mint_account.as_ref().unwrap().data).unwrap();
    let token_1_mint_info =
        common_utils::unpack_mint(&token_1_mint_account.as_ref().unwrap().data).unwrap();
    let user_input_token_info =
        common_utils::unpack_token(&user_input_token_account.as_ref().unwrap().data).unwrap();

    let (total_token_0_amount, total_token_1_amount) = pool_state.vault_amount_without_fee(
        token_0_vault_info.base.amount,
        token_1_vault_info.base.amount,
    );

    let (
        trade_direction,
        total_input_token_amount,
        total_output_token_amount,
        input_vault,
        output_vault,
        input_mint,
        output_mint,
        input_token_program,
        output_token_program,
        transfer_fee,
    ) = if user_input_token_info.base.mint == token_0_vault_info.base.mint {
        (
            raydium_cp_swap::curve::TradeDirection::ZeroForOne,
            total_token_0_amount,
            total_token_1_amount,
            pool_state.token_0_vault,
            pool_state.token_1_vault,
            pool_state.token_0_mint,
            pool_state.token_1_mint,
            pool_state.token_0_program,
            pool_state.token_1_program,
            if base_in {
                common_utils::get_transfer_fee(&token_0_mint_info, epoch, amount_specified)
            } else {
                common_utils::get_transfer_inverse_fee(&token_1_mint_info, epoch, amount_specified)
            },
        )
    } else if user_input_token_info.base.mint == token_1_vault_info.base.mint {
        (
            raydium_cp_swap::curve::TradeDirection::OneForZero,
            total_token_1_amount,
            total_token_0_amount,
            pool_state.token_1_vault,
            pool_state.token_0_vault,
            pool_state.token_1_mint,
            pool_state.token_0_mint,
            pool_state.token_1_program,
            pool_state.token_0_program,
            if base_in {
                common_utils::get_transfer_fee(&token_1_mint_info, epoch, amount_specified)
            } else {
                common_utils::get_transfer_inverse_fee(&token_0_mint_info, epoch, amount_specified)
            },
        )
    } else {
        panic!("input tokens not match pool vaults");
    };

    let other_amount_threshold = if base_in {
        // Take transfer fees into account for actual amount transferred in
        let actual_amount_in = amount_specified.saturating_sub(transfer_fee);
        let result = raydium_cp_swap::curve::CurveCalculator::swap_base_input(
            u128::from(actual_amount_in),
            u128::from(total_input_token_amount),
            u128::from(total_output_token_amount),
            amm_config_state.trade_fee_rate,
            amm_config_state.protocol_fee_rate,
            amm_config_state.fund_fee_rate,
        )
        .ok_or(raydium_cp_swap::error::ErrorCode::ZeroTradingTokens)
        .unwrap();
        let amount_out = u64::try_from(result.destination_amount_swapped).unwrap();
        let transfer_fee = match trade_direction {
            raydium_cp_swap::curve::TradeDirection::ZeroForOne => {
                common_utils::get_transfer_fee(&token_1_mint_info, epoch, amount_out)
            }
            raydium_cp_swap::curve::TradeDirection::OneForZero => {
                common_utils::get_transfer_fee(&token_0_mint_info, epoch, amount_out)
            }
        };
        let amount_received = amount_out.checked_sub(transfer_fee).unwrap();
        // calc mint out amount with slippage
        let minimum_amount_out =
            common_utils::amount_with_slippage(amount_received, slippage_bps, false)?;
        minimum_amount_out
    } else {
        // Take transfer fees into account for actual amount user received
        let actual_amount_out = amount_specified.checked_add(transfer_fee).unwrap();

        let result = raydium_cp_swap::curve::CurveCalculator::swap_base_output(
            u128::from(actual_amount_out),
            u128::from(total_input_token_amount),
            u128::from(total_output_token_amount),
            amm_config_state.trade_fee_rate,
            amm_config_state.protocol_fee_rate,
            amm_config_state.fund_fee_rate,
        )
        .ok_or(raydium_cp_swap::error::ErrorCode::ZeroTradingTokens)
        .unwrap();

        let source_amount_swapped = u64::try_from(result.source_amount_swapped).unwrap();
        let amount_in_transfer_fee = match trade_direction {
            raydium_cp_swap::curve::TradeDirection::ZeroForOne => {
                common_utils::get_transfer_inverse_fee(
                    &token_0_mint_info,
                    epoch,
                    source_amount_swapped,
                )
            }
            raydium_cp_swap::curve::TradeDirection::OneForZero => {
                common_utils::get_transfer_inverse_fee(
                    &token_1_mint_info,
                    epoch,
                    source_amount_swapped,
                )
            }
        };
        let input_transfer_amount = source_amount_swapped
            .checked_add(amount_in_transfer_fee)
            .unwrap();
        // calc max in with slippage
        let max_amount_in =
            common_utils::amount_with_slippage(input_transfer_amount, slippage_bps, true)?;
        max_amount_in
    };

    Ok(CpSwapSwapChangeResult {
        pool_id,
        pool_config: pool_state.amm_config,
        pool_observation: pool_state.observation_key,
        user_input_token,
        input_vault,
        output_vault,
        input_mint,
        output_mint,
        input_token_program,
        output_token_program,
        amount_specified,
        other_amount_threshold,
    })
}
