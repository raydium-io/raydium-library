use anyhow::Result;
use common::common_utils;
use raydium_amm::math::{CheckedCeilDiv, U128};

pub fn pool_vault_deduct_pnl(
    pc_vault_amount_with_pnl: u64,
    coin_vault_amount_with_pnl: u64,
    amm: &mut raydium_amm::state::AmmInfo,
    target: &raydium_amm::state::TargetOrders,
) -> Result<(u64, u64)> {
    let mut pc_vault_amount_with_pnl = pc_vault_amount_with_pnl;
    let mut coin_vault_amount_with_pnl = coin_vault_amount_with_pnl;
    let x = raydium_amm::math::Calculator::normalize_decimal_v2(
        pc_vault_amount_with_pnl,
        amm.pc_decimals,
        amm.sys_decimal_value,
    );
    let y = raydium_amm::math::Calculator::normalize_decimal_v2(
        coin_vault_amount_with_pnl,
        amm.coin_decimals,
        amm.sys_decimal_value,
    );
    // calc and update pnl with adjust vault amount
    let (_delta_x, _delta_y) = raydium_amm::processor::Processor::calc_take_pnl(
        target,
        amm,
        &mut pc_vault_amount_with_pnl,
        &mut coin_vault_amount_with_pnl,
        x.as_u128().into(),
        y.as_u128().into(),
    )
    .unwrap();

    Ok((pc_vault_amount_with_pnl, coin_vault_amount_with_pnl))
}

fn deposit_exact_amount(
    pc_vault_amount_without_pnl: u64,
    coin_vault_amount_without_pnl: u64,
    input_amount: u64,
    base_side: u64,
) -> Result<u64> {
    // calc deposit amount
    let invariant = raydium_amm::math::InvariantToken {
        token_coin: coin_vault_amount_without_pnl,
        token_pc: pc_vault_amount_without_pnl,
    };
    match base_side {
        0 => {
            // input amount is coin
            let another_amount = invariant
                .exchange_coin_to_pc(input_amount, raydium_amm::math::RoundDirection::Ceiling)
                .unwrap();
            Ok(another_amount)
        }
        _ => {
            // input amount is pc
            let another_amount = invariant
                .exchange_pc_to_coin(input_amount, raydium_amm::math::RoundDirection::Ceiling)
                .unwrap();
            Ok(another_amount)
        }
    }
}

fn withdraw_exact_amounts(
    pc_vault_amount_without_pnl: u64,
    coin_vault_amount_without_pnl: u64,
    pool_lp_amount: u64,
    withdraw_lp_amount: u64,
) -> Result<(u64, u64)> {
    // calc withdraw amount
    let invariant = raydium_amm::math::InvariantPool {
        token_input: withdraw_lp_amount,
        token_total: pool_lp_amount,
    };
    let pc_amount = invariant
        .exchange_pool_to_token(
            pc_vault_amount_without_pnl,
            raydium_amm::math::RoundDirection::Floor,
        )
        .unwrap();
    let coin_amount = invariant
        .exchange_pool_to_token(
            coin_vault_amount_without_pnl,
            raydium_amm::math::RoundDirection::Floor,
        )
        .unwrap();

    Ok((pc_amount, coin_amount))
}

fn swap_exact_amount(
    pc_vault_amount: u64,
    coin_vault_amount: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    swap_direction: raydium_amm::math::SwapDirection,
    amount_specified: u64,
    swap_base_in: bool,
) -> Result<u64> {
    let other_amount_threshold = if swap_base_in {
        let swap_fee = U128::from(amount_specified)
            .checked_mul(swap_fee_numerator.into())
            .unwrap()
            .checked_ceil_div(swap_fee_denominator.into())
            .unwrap()
            .0;
        let swap_in_after_deduct_fee = U128::from(amount_specified).checked_sub(swap_fee).unwrap();
        let swap_amount_out = raydium_amm::math::Calculator::swap_token_amount_base_in(
            swap_in_after_deduct_fee,
            pc_vault_amount.into(),
            coin_vault_amount.into(),
            swap_direction,
        )
        .as_u64();
        swap_amount_out
    } else {
        let swap_in_before_add_fee = raydium_amm::math::Calculator::swap_token_amount_base_out(
            amount_specified.into(),
            pc_vault_amount.into(),
            coin_vault_amount.into(),
            swap_direction,
        );
        let swap_in_after_add_fee = swap_in_before_add_fee
            .checked_mul(swap_fee_denominator.into())
            .unwrap()
            .checked_ceil_div(
                (swap_fee_denominator
                    .checked_sub(swap_fee_numerator)
                    .unwrap())
                .into(),
            )
            .unwrap()
            .0
            .as_u64();

        swap_in_after_add_fee
    };

    Ok(other_amount_threshold)
}

pub fn deposit_amount_with_slippage(
    pc_vault_amount_without_pnl: u64,
    coin_vault_amount_without_pnl: u64,
    amount_specified: u64,
    another_min_limit: bool,
    base_side: u64,
    slippage_bps: u64,
) -> Result<(u64, u64, Option<u64>)> {
    let another_amount = deposit_exact_amount(
        pc_vault_amount_without_pnl,
        coin_vault_amount_without_pnl,
        amount_specified,
        base_side,
    )?;
    match base_side {
        0 => {
            let max_coin_amout = amount_specified;
            let max_pc_amount =
                common_utils::amount_with_slippage(another_amount, slippage_bps, true)?;
            let min_pc_amount = if another_min_limit {
                Some(common_utils::amount_with_slippage(
                    another_amount,
                    slippage_bps,
                    false,
                )?)
            } else {
                None
            };
            return Ok((max_coin_amout, max_pc_amount, min_pc_amount));
        }
        _ => {
            let max_coin_amount =
                common_utils::amount_with_slippage(another_amount, slippage_bps, true)?;
            let min_coin_amount = if another_min_limit {
                Some(common_utils::amount_with_slippage(
                    another_amount,
                    slippage_bps,
                    false,
                )?)
            } else {
                None
            };
            let max_pc_amount = amount_specified;
            return Ok((max_coin_amount, max_pc_amount, min_coin_amount));
        }
    }
}

pub fn withdraw_amounts_with_slippage(
    pc_vault_amount_without_pnl: u64,
    coin_vault_amount_without_pnl: u64,
    pool_lp_amount: u64,
    withdraw_lp_amount: u64,
    slippage_bps_opt: Option<u64>,
) -> Result<(Option<u64>, Option<u64>)> {
    let (receive_min_coin_amount, receive_min_pc_amount) =
        if let Some(slippage_bps) = slippage_bps_opt {
            let (receive_pc_amount, receive_coin_amount) = withdraw_exact_amounts(
                pc_vault_amount_without_pnl,
                coin_vault_amount_without_pnl,
                pool_lp_amount,
                withdraw_lp_amount,
            )?;
            let receive_min_pc_amount =
                common_utils::amount_with_slippage(receive_pc_amount, slippage_bps, false)?;
            let receive_min_coin_amount =
                common_utils::amount_with_slippage(receive_coin_amount, slippage_bps, false)?;
            (Some(receive_min_coin_amount), Some(receive_min_pc_amount))
        } else {
            (None, None)
        };

    Ok((receive_min_coin_amount, receive_min_pc_amount))
}

pub fn swap_with_slippage(
    pc_vault_amount: u64,
    coin_vault_amount: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    swap_direction: raydium_amm::math::SwapDirection,
    amount_specified: u64,
    swap_base_in: bool,
    slippage_bps: u64,
) -> Result<u64> {
    let other_amount_threshold = swap_exact_amount(
        pc_vault_amount,
        coin_vault_amount,
        swap_fee_numerator,
        swap_fee_denominator,
        swap_direction,
        amount_specified,
        swap_base_in,
    )?;
    let other_amount_threshold = if swap_base_in {
        // min out
        common_utils::amount_with_slippage(other_amount_threshold, slippage_bps, false)?
    } else {
        // max in
        common_utils::amount_with_slippage(other_amount_threshold, slippage_bps, true)?
    };
    Ok(other_amount_threshold)
}
