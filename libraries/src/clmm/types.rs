use solana_sdk::pubkey::Pubkey;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub struct RewardItem {
    pub token_program: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_vault: Pubkey,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClmmCreatePoolResult {
    pub mint0: Pubkey,
    pub mint1: Pubkey,
    pub mint0_token_program: Pubkey,
    pub mint1_token_program: Pubkey,
    pub price: f64,
    pub sqrt_price_x64: u128,
    pub tick: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClmmLiquidityChangeResult {
    pub mint0: Pubkey,
    pub mint1: Pubkey,
    pub vault0: Pubkey,
    pub vault1: Pubkey,
    pub mint0_token_program: Pubkey,
    pub mint1_token_program: Pubkey,
    pub reward_items: Vec<RewardItem>,
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub tick_array_lower_start_index: i32,
    pub tick_array_upper_start_index: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClmmSwapChangeResult {
    pub pool_amm_config: Pubkey,
    pub pool_id: Pubkey,
    pub pool_observation: Pubkey,
    pub input_vault: Pubkey,
    pub output_vault: Pubkey,
    pub input_vault_mint: Pubkey,
    pub output_vault_mint: Pubkey,
    pub input_token_program: Pubkey,
    pub output_token_program: Pubkey,
    pub user_input_token: Pubkey,
    pub remaining_tick_array_keys: VecDeque<Pubkey>,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit_x64: Option<u128>,
    pub is_base_input: bool,
}

// the top level state of the swap, the results of which are recorded in storage at the end
#[derive(Debug)]
pub struct SwapState {
    // the amount remaining to be swapped in/out of the input/output asset
    pub amount_specified_remaining: u64,
    // the amount already swapped out/in of the output/input asset
    pub amount_calculated: u64,
    // current sqrt(price)
    pub sqrt_price_x64: u128,
    // the tick associated with the current price
    pub tick: i32,
    // the current liquidity in range
    pub liquidity: u128,
}
#[derive(Default)]
pub struct StepComputations {
    // the price at the beginning of the step
    pub sqrt_price_start_x64: u128,
    // the next tick to swap to from the current tick in the swap direction
    pub tick_next: i32,
    // whether tick_next is initialized or not
    pub initialized: bool,
    // sqrt(price) for the next tick (1/0)
    pub sqrt_price_next_x64: u128,
    // how much is being swapped in in this step
    pub amount_in: u64,
    // how much is being swapped out
    pub amount_out: u64,
    // how much fee is being paid in
    pub fee_amount: u64,
}
