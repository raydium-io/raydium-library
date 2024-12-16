use crate::{
    clmm_math,
    clmm_types::{
        ClmmCreatePoolResult, ClmmLiquidityChangeResult, ClmmSwapChangeResult, RewardItem,
        StepComputations, SwapState,
    },
};
use anyhow::Result;
use arrayref::array_ref;
use common::{common_types::TokenInfo, common_utils, rpc};
use raydium_amm_v3::libraries::{liquidity_math, tick_math};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::VecDeque,
    ops::{DerefMut, Neg},
};

pub fn create_pool_price(
    rpc_client: &RpcClient,
    mint0: Pubkey,
    mint1: Pubkey,
    price: f64,
) -> Result<ClmmCreatePoolResult> {
    let mut price = price;
    let mut mint0 = mint0;
    let mut mint1 = mint1;
    if mint0 > mint1 {
        std::mem::swap(&mut mint0, &mut mint1);
        price = 1.0 / price;
    }
    println!("mint0:{}, mint1:{}, price:{}", mint0, mint1, price);
    let load_pubkeys = vec![mint0, mint1];
    let rsps = rpc_client.get_multiple_accounts(&load_pubkeys).unwrap();
    let mint0_token_program = rsps[0].as_ref().unwrap().owner;
    let mint1_token_program = rsps[1].as_ref().unwrap().owner;
    let mint0_info = common_utils::unpack_mint(&rsps[0].as_ref().unwrap().data).unwrap();
    let mint1_info = common_utils::unpack_mint(&rsps[1].as_ref().unwrap().data).unwrap();
    let sqrt_price_x64 = clmm_math::price_to_sqrt_price_x64(
        price,
        mint0_info.base.decimals,
        mint1_info.base.decimals,
    );
    let tick = tick_math::get_tick_at_sqrt_price(sqrt_price_x64).unwrap();
    Ok(ClmmCreatePoolResult {
        mint0,
        mint1,
        mint0_token_program,
        mint1_token_program,
        price,
        sqrt_price_x64,
        tick,
    })
}

pub fn calculate_liquidity_change(
    rpc_client: &RpcClient,
    pool_id: Pubkey,
    tick_lower_price: f64,
    tick_upper_price: f64,
    input_amount: u64,
    slippage_bps: u64,
    collect_reward: bool,
    is_base_0: bool,
) -> Result<ClmmLiquidityChangeResult> {
    let pool = rpc::get_anchor_account::<raydium_amm_v3::states::PoolState>(rpc_client, &pool_id)
        .unwrap()
        .unwrap();
    let mut load_pubkeys = vec![pool.token_mint_0, pool.token_mint_1];

    let mut reward_items: Vec<RewardItem> = Vec::new();
    if collect_reward {
        // collect reward info while decrease liquidity
        for item in pool.reward_infos.iter() {
            if item.token_mint != Pubkey::default() {
                reward_items.push(RewardItem {
                    token_program: Pubkey::default(),
                    reward_mint: item.token_mint,
                    reward_vault: item.token_vault,
                });
                load_pubkeys.push(item.token_mint);
            }
        }
    }
    let mut rsps = rpc_client.get_multiple_accounts(&load_pubkeys).unwrap();
    let mint0_token_program = rsps.remove(0).unwrap().owner;
    let mint1_token_program = rsps.remove(0).unwrap().owner;
    for (item, rsp) in reward_items.iter_mut().zip(rsps.iter()) {
        item.token_program = rsp.as_ref().unwrap().owner;
    }

    let tick_lower_price_x64 = clmm_math::price_to_sqrt_price_x64(
        tick_lower_price,
        pool.mint_decimals_0,
        pool.mint_decimals_1,
    );
    let tick_upper_price_x64 = clmm_math::price_to_sqrt_price_x64(
        tick_upper_price,
        pool.mint_decimals_0,
        pool.mint_decimals_1,
    );
    let tick_lower_index = clmm_math::tick_with_spacing(
        tick_math::get_tick_at_sqrt_price(tick_lower_price_x64)?,
        pool.tick_spacing.into(),
    );
    let tick_upper_index = clmm_math::tick_with_spacing(
        tick_math::get_tick_at_sqrt_price(tick_upper_price_x64)?,
        pool.tick_spacing.into(),
    );
    println!(
        "tick_lower_index:{}, tick_upper_index:{}",
        tick_lower_index, tick_upper_index
    );
    let tick_lower_price_x64 = tick_math::get_sqrt_price_at_tick(tick_lower_index)?;
    let tick_upper_price_x64 = tick_math::get_sqrt_price_at_tick(tick_upper_index)?;
    let liquidity = if is_base_0 {
        liquidity_math::get_liquidity_from_single_amount_0(
            pool.sqrt_price_x64,
            tick_lower_price_x64,
            tick_upper_price_x64,
            input_amount,
        )
    } else {
        liquidity_math::get_liquidity_from_single_amount_1(
            pool.sqrt_price_x64,
            tick_lower_price_x64,
            tick_upper_price_x64,
            input_amount,
        )
    };
    let (amount_0, amount_1) = liquidity_math::get_delta_amounts_signed(
        pool.tick_current,
        pool.sqrt_price_x64,
        tick_lower_index,
        tick_upper_index,
        liquidity as i128,
    )?;
    println!(
        "amount_0:{}, amount_1:{}, liquidity:{}",
        amount_0, amount_1, liquidity
    );
    // calc with slippage
    let amount_0_with_slippage = common_utils::amount_with_slippage(amount_0, slippage_bps, true)?;
    let amount_1_with_slippage = common_utils::amount_with_slippage(amount_1, slippage_bps, true)?;
    // calc with transfer_fee
    let transfer_fee = common_utils::get_pool_mints_inverse_fee(
        &rpc_client,
        pool.token_mint_0,
        pool.token_mint_1,
        amount_0_with_slippage,
        amount_1_with_slippage,
    );
    println!(
        "transfer_fee_0:{}, transfer_fee_1:{}",
        transfer_fee.0.transfer_fee, transfer_fee.1.transfer_fee
    );
    let amount_0_max = amount_0_with_slippage
        .checked_add(transfer_fee.0.transfer_fee)
        .unwrap();
    let amount_1_max = amount_1_with_slippage
        .checked_add(transfer_fee.1.transfer_fee)
        .unwrap();

    let tick_array_lower_start_index =
        raydium_amm_v3::states::TickArrayState::get_array_start_index(
            tick_lower_index,
            pool.tick_spacing.into(),
        );
    let tick_array_upper_start_index =
        raydium_amm_v3::states::TickArrayState::get_array_start_index(
            tick_upper_index,
            pool.tick_spacing.into(),
        );
    Ok(ClmmLiquidityChangeResult {
        mint0: pool.token_mint_0,
        mint1: pool.token_mint_1,
        vault0: pool.token_vault_0,
        vault1: pool.token_vault_1,
        mint0_token_program,
        mint1_token_program,
        reward_items,
        liquidity,
        amount_0: amount_0_max,
        amount_1: amount_1_max,
        tick_lower_index,
        tick_upper_index,
        tick_array_lower_start_index,
        tick_array_upper_start_index,
    })
}

pub fn calculate_swap_change(
    rpc_client: &RpcClient,
    raydium_v3_program: Pubkey,
    pool_id: Pubkey,
    tickarray_bitmap_extension: Pubkey,
    input_token: Pubkey,
    amount: u64,
    limit_price: Option<f64>,
    base_in: bool,
    slippage_bps: u64,
) -> Result<ClmmSwapChangeResult> {
    let pool_state =
        rpc::get_anchor_account::<raydium_amm_v3::states::PoolState>(rpc_client, &pool_id)
            .unwrap()
            .unwrap();
    // load mult account
    let load_accounts = vec![
        input_token,
        pool_state.amm_config,
        pool_state.token_mint_0,
        pool_state.token_mint_1,
        tickarray_bitmap_extension,
    ];
    let rsps = rpc_client.get_multiple_accounts(&load_accounts).unwrap();
    let epoch = rpc_client.get_epoch_info().unwrap().epoch;
    let [user_input_account, amm_config_account, mint0_account, mint1_account, tickarray_bitmap_extension_account] =
        array_ref![rsps, 0, 5];
    let mint0_token_program = mint0_account.as_ref().unwrap().owner;
    let mint1_token_program = mint1_account.as_ref().unwrap().owner;
    let user_input_state =
        common_utils::unpack_token(&user_input_account.as_ref().unwrap().data).unwrap();
    let mint0_state = common_utils::unpack_mint(&mint0_account.as_ref().unwrap().data).unwrap();
    let mint1_state = common_utils::unpack_mint(&mint1_account.as_ref().unwrap().data).unwrap();
    let tickarray_bitmap_extension_state = common_utils::deserialize_anchor_account::<
        raydium_amm_v3::states::TickArrayBitmapExtension,
    >(
        tickarray_bitmap_extension_account.as_ref().unwrap()
    )
    .unwrap();
    let amm_config_state = common_utils::deserialize_anchor_account::<
        raydium_amm_v3::states::AmmConfig,
    >(amm_config_account.as_ref().unwrap())
    .unwrap();

    let (
        zero_for_one,
        input_vault,
        output_vault,
        input_vault_mint,
        output_vault_mint,
        input_token_program,
        output_token_program,
    ) = if user_input_state.base.mint == pool_state.token_mint_0 {
        (
            true,
            pool_state.token_vault_0,
            pool_state.token_vault_1,
            pool_state.token_mint_0,
            pool_state.token_mint_1,
            mint0_token_program,
            mint1_token_program,
        )
    } else if user_input_state.base.mint == pool_state.token_mint_1 {
        (
            false,
            pool_state.token_vault_1,
            pool_state.token_vault_0,
            pool_state.token_mint_1,
            pool_state.token_mint_0,
            mint1_token_program,
            mint0_token_program,
        )
    } else {
        panic!("input tokens not match pool vaults");
    };
    let transfer_fee = if base_in {
        if zero_for_one {
            common_utils::get_transfer_fee(&mint0_state, epoch, amount)
        } else {
            common_utils::get_transfer_fee(&mint1_state, epoch, amount)
        }
    } else {
        0
    };
    let amount_specified = amount.checked_sub(transfer_fee).unwrap();
    // load tick_arrays
    let mut tick_arrays = load_cur_and_next_five_tick_array(
        rpc_client,
        raydium_v3_program,
        pool_id,
        &pool_state,
        &tickarray_bitmap_extension_state,
        zero_for_one,
    );
    let sqrt_price_limit_x64 = if limit_price.is_some() {
        let sqrt_price_x64 = clmm_math::price_to_sqrt_price_x64(
            limit_price.unwrap(),
            pool_state.mint_decimals_0,
            pool_state.mint_decimals_1,
        );
        Some(sqrt_price_x64)
    } else {
        None
    };

    let (mut other_amount_threshold, tick_array_indexs) =
        get_out_put_amount_and_remaining_accounts(
            amount_specified,
            sqrt_price_limit_x64,
            zero_for_one,
            base_in,
            amm_config_state.trade_fee_rate,
            &pool_state,
            &tickarray_bitmap_extension_state,
            &mut tick_arrays,
        )
        .unwrap();
    println!(
        "amount:{}, other_amount_threshold:{}",
        amount, other_amount_threshold
    );
    let remaining_tick_array_keys = tick_array_indexs
        .into_iter()
        .map(|index| {
            Pubkey::find_program_address(
                &[
                    raydium_amm_v3::states::TICK_ARRAY_SEED.as_bytes(),
                    pool_id.to_bytes().as_ref(),
                    &index.to_be_bytes(),
                ],
                &raydium_v3_program,
            )
            .0
        })
        .collect();
    if base_in {
        // calc mint out amount with slippage
        other_amount_threshold =
            common_utils::amount_with_slippage(other_amount_threshold, slippage_bps, false)?;
    } else {
        // calc max in with slippage
        other_amount_threshold =
            common_utils::amount_with_slippage(other_amount_threshold, slippage_bps, true)?;
        // calc max in with transfer_fee
        let transfer_fee = if zero_for_one {
            common_utils::get_transfer_inverse_fee(&mint0_state, epoch, other_amount_threshold)
        } else {
            common_utils::get_transfer_inverse_fee(&mint1_state, epoch, other_amount_threshold)
        };
        other_amount_threshold += transfer_fee;
    }
    Ok(ClmmSwapChangeResult {
        pool_amm_config: pool_state.amm_config,
        pool_id,
        pool_observation: pool_state.observation_key,
        input_vault,
        output_vault,
        input_vault_mint,
        output_vault_mint,
        input_token_program,
        output_token_program,
        user_input_token: input_token,
        remaining_tick_array_keys,
        amount,
        other_amount_threshold,
        sqrt_price_limit_x64,
        is_base_input: base_in,
    })
}

fn load_cur_and_next_five_tick_array(
    rpc_client: &RpcClient,
    raydium_v3_program: Pubkey,
    pool_id: Pubkey,
    pool_state: &raydium_amm_v3::states::PoolState,
    tickarray_bitmap_extension: &raydium_amm_v3::states::TickArrayBitmapExtension,
    zero_for_one: bool,
) -> VecDeque<raydium_amm_v3::states::TickArrayState> {
    let (_, mut current_vaild_tick_array_start_index) = pool_state
        .get_first_initialized_tick_array(&Some(*tickarray_bitmap_extension), zero_for_one)
        .unwrap();
    let mut tick_array_keys = Vec::new();
    tick_array_keys.push(
        Pubkey::find_program_address(
            &[
                raydium_amm_v3::states::TICK_ARRAY_SEED.as_bytes(),
                pool_id.to_bytes().as_ref(),
                &current_vaild_tick_array_start_index.to_be_bytes(),
            ],
            &raydium_v3_program,
        )
        .0,
    );
    let mut max_array_size = 5;
    while max_array_size != 0 {
        let next_tick_array_index = pool_state
            .next_initialized_tick_array_start_index(
                &Some(*tickarray_bitmap_extension),
                current_vaild_tick_array_start_index,
                zero_for_one,
            )
            .unwrap();
        if next_tick_array_index.is_none() {
            break;
        }
        current_vaild_tick_array_start_index = next_tick_array_index.unwrap();
        tick_array_keys.push(
            Pubkey::find_program_address(
                &[
                    raydium_amm_v3::states::TICK_ARRAY_SEED.as_bytes(),
                    pool_id.to_bytes().as_ref(),
                    &current_vaild_tick_array_start_index.to_be_bytes(),
                ],
                &raydium_v3_program,
            )
            .0,
        );
        max_array_size -= 1;
    }
    let tick_array_rsps = rpc_client.get_multiple_accounts(&tick_array_keys).unwrap();
    let mut tick_arrays = VecDeque::new();
    for tick_array in tick_array_rsps {
        let tick_array_state = common_utils::deserialize_anchor_account::<
            raydium_amm_v3::states::TickArrayState,
        >(&tick_array.unwrap())
        .unwrap();
        tick_arrays.push_back(tick_array_state);
    }
    tick_arrays
}

pub fn get_out_put_amount_and_remaining_accounts(
    input_amount: u64,
    sqrt_price_limit_x64: Option<u128>,
    zero_for_one: bool,
    is_base_input: bool,
    trade_fee_rate: u32,
    pool_state: &raydium_amm_v3::states::PoolState,
    tickarray_bitmap_extension: &raydium_amm_v3::states::TickArrayBitmapExtension,
    tick_arrays: &mut VecDeque<raydium_amm_v3::states::TickArrayState>,
) -> Result<(u64, VecDeque<i32>), &'static str> {
    let (is_pool_current_tick_array, current_vaild_tick_array_start_index) = pool_state
        .get_first_initialized_tick_array(&Some(*tickarray_bitmap_extension), zero_for_one)
        .unwrap();

    let (amount_calculated, tick_array_start_index_vec) = swap_compute(
        zero_for_one,
        is_base_input,
        is_pool_current_tick_array,
        trade_fee_rate,
        input_amount,
        current_vaild_tick_array_start_index,
        sqrt_price_limit_x64.unwrap_or(0),
        pool_state,
        tickarray_bitmap_extension,
        tick_arrays,
    )?;
    println!("tick_array_start_index:{:?}", tick_array_start_index_vec);

    Ok((amount_calculated, tick_array_start_index_vec))
}

fn swap_compute(
    zero_for_one: bool,
    is_base_input: bool,
    is_pool_current_tick_array: bool,
    trade_fee_rate: u32,
    amount_specified: u64,
    current_vaild_tick_array_start_index: i32,
    sqrt_price_limit_x64: u128,
    pool_state: &raydium_amm_v3::states::PoolState,
    tickarray_bitmap_extension: &raydium_amm_v3::states::TickArrayBitmapExtension,
    tick_arrays: &mut VecDeque<raydium_amm_v3::states::TickArrayState>,
) -> Result<(u64, VecDeque<i32>), &'static str> {
    if amount_specified == 0 {
        return Result::Err("amountSpecified must not be 0");
    }
    let sqrt_price_limit_x64 = if sqrt_price_limit_x64 == 0 {
        if zero_for_one {
            tick_math::MIN_SQRT_PRICE_X64 + 1
        } else {
            tick_math::MAX_SQRT_PRICE_X64 - 1
        }
    } else {
        sqrt_price_limit_x64
    };
    if zero_for_one {
        if sqrt_price_limit_x64 < tick_math::MIN_SQRT_PRICE_X64 {
            return Result::Err("sqrt_price_limit_x64 must greater than MIN_SQRT_PRICE_X64");
        }
        if sqrt_price_limit_x64 >= pool_state.sqrt_price_x64 {
            return Result::Err("sqrt_price_limit_x64 must smaller than current");
        }
    } else {
        if sqrt_price_limit_x64 > tick_math::MAX_SQRT_PRICE_X64 {
            return Result::Err("sqrt_price_limit_x64 must smaller than MAX_SQRT_PRICE_X64");
        }
        if sqrt_price_limit_x64 <= pool_state.sqrt_price_x64 {
            return Result::Err("sqrt_price_limit_x64 must greater than current");
        }
    }
    let mut tick_match_current_tick_array = is_pool_current_tick_array;

    let mut state = SwapState {
        amount_specified_remaining: amount_specified,
        amount_calculated: 0,
        sqrt_price_x64: pool_state.sqrt_price_x64,
        tick: pool_state.tick_current,
        liquidity: pool_state.liquidity,
    };

    let mut tick_array_current = tick_arrays.pop_front().unwrap();
    if tick_array_current.start_tick_index != current_vaild_tick_array_start_index {
        return Result::Err("tick array start tick index does not match");
    }
    let mut tick_array_start_index_vec = VecDeque::new();
    tick_array_start_index_vec.push_back(tick_array_current.start_tick_index);
    let mut loop_count = 0;
    // loop across ticks until input liquidity is consumed, or the limit price is reached
    while state.amount_specified_remaining != 0
        && state.sqrt_price_x64 != sqrt_price_limit_x64
        && state.tick < tick_math::MAX_TICK
        && state.tick > tick_math::MIN_TICK
    {
        if loop_count > 10 {
            return Result::Err("loop_count limit");
        }
        let mut step = StepComputations::default();
        step.sqrt_price_start_x64 = state.sqrt_price_x64;
        // save the bitmap, and the tick account if it is initialized
        let mut next_initialized_tick = if let Some(tick_state) = tick_array_current
            .next_initialized_tick(state.tick, pool_state.tick_spacing, zero_for_one)
            .unwrap()
        {
            Box::new(*tick_state)
        } else {
            if !tick_match_current_tick_array {
                tick_match_current_tick_array = true;
                Box::new(
                    *tick_array_current
                        .first_initialized_tick(zero_for_one)
                        .unwrap(),
                )
            } else {
                Box::new(raydium_amm_v3::states::TickState::default())
            }
        };
        if !next_initialized_tick.is_initialized() {
            let current_vaild_tick_array_start_index = pool_state
                .next_initialized_tick_array_start_index(
                    &Some(*tickarray_bitmap_extension),
                    current_vaild_tick_array_start_index,
                    zero_for_one,
                )
                .unwrap();
            tick_array_current = tick_arrays.pop_front().unwrap();
            if current_vaild_tick_array_start_index.is_none() {
                return Result::Err("tick array start tick index out of range limit");
            }
            if tick_array_current.start_tick_index != current_vaild_tick_array_start_index.unwrap()
            {
                return Result::Err("tick array start tick index does not match");
            }
            tick_array_start_index_vec.push_back(tick_array_current.start_tick_index);
            let mut first_initialized_tick = tick_array_current
                .first_initialized_tick(zero_for_one)
                .unwrap();

            next_initialized_tick = Box::new(*first_initialized_tick.deref_mut());
        }
        step.tick_next = next_initialized_tick.tick;
        step.initialized = next_initialized_tick.is_initialized();
        if step.tick_next < tick_math::MIN_TICK {
            step.tick_next = tick_math::MIN_TICK;
        } else if step.tick_next > tick_math::MAX_TICK {
            step.tick_next = tick_math::MAX_TICK;
        }

        step.sqrt_price_next_x64 = tick_math::get_sqrt_price_at_tick(step.tick_next).unwrap();

        let target_price = if (zero_for_one && step.sqrt_price_next_x64 < sqrt_price_limit_x64)
            || (!zero_for_one && step.sqrt_price_next_x64 > sqrt_price_limit_x64)
        {
            sqrt_price_limit_x64
        } else {
            step.sqrt_price_next_x64
        };
        let swap_step = raydium_amm_v3::libraries::swap_math::compute_swap_step(
            state.sqrt_price_x64,
            target_price,
            state.liquidity,
            state.amount_specified_remaining,
            trade_fee_rate,
            is_base_input,
            zero_for_one,
            1,
        )
        .unwrap();
        state.sqrt_price_x64 = swap_step.sqrt_price_next_x64;
        step.amount_in = swap_step.amount_in;
        step.amount_out = swap_step.amount_out;
        step.fee_amount = swap_step.fee_amount;

        if is_base_input {
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(step.amount_in + step.fee_amount)
                .unwrap();
            state.amount_calculated = state
                .amount_calculated
                .checked_add(step.amount_out)
                .unwrap();
        } else {
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(step.amount_out)
                .unwrap();
            state.amount_calculated = state
                .amount_calculated
                .checked_add(step.amount_in + step.fee_amount)
                .unwrap();
        }

        if state.sqrt_price_x64 == step.sqrt_price_next_x64 {
            // if the tick is initialized, run the tick transition
            if step.initialized {
                let mut liquidity_net = next_initialized_tick.liquidity_net;
                if zero_for_one {
                    liquidity_net = liquidity_net.neg();
                }
                state.liquidity =
                    liquidity_math::add_delta(state.liquidity, liquidity_net).unwrap();
            }

            state.tick = if zero_for_one {
                step.tick_next - 1
            } else {
                step.tick_next
            };
        } else if state.sqrt_price_x64 != step.sqrt_price_start_x64 {
            // recompute unless we're on a lower tick boundary (i.e. already transitioned ticks), and haven't moved
            state.tick = tick_math::get_tick_at_sqrt_price(state.sqrt_price_x64).unwrap();
        }
        loop_count += 1;
    }

    Ok((state.amount_calculated, tick_array_start_index_vec))
}

pub fn get_nft_accounts_and_positions_by_owner(
    client: &RpcClient,
    owner: &Pubkey,
    raydium_amm_v3_program: &Pubkey,
) -> (Vec<TokenInfo>, Vec<Pubkey>) {
    let mut nft_accounts_info = common_utils::get_nft_accounts_by_owner_with_specified_program(
        client,
        owner,
        spl_token::id(),
    );
    let spl_2022_nfts = common_utils::get_nft_accounts_by_owner_with_specified_program(
        client,
        owner,
        spl_token_2022::id(),
    );
    nft_accounts_info.extend(spl_2022_nfts);
    let user_position_account: Vec<Pubkey> = nft_accounts_info
        .iter()
        .map(|&nft| {
            Pubkey::find_program_address(
                &[
                    raydium_amm_v3::states::POSITION_SEED.as_bytes(),
                    nft.mint.to_bytes().as_ref(),
                ],
                &raydium_amm_v3_program,
            )
            .0
        })
        .collect();
    (nft_accounts_info, user_position_account)
}
