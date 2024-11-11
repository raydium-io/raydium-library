use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Copy, Debug)]
pub struct AmmKeys {
    pub amm_pool: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_target: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_open_order: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub nonce: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct CalculateResult {
    pub pool_pc_vault_amount: u64,
    pub pool_coin_vault_amount: u64,
    pub pool_lp_amount: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmmDepositInfoResult {
    pub pool_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub market: Pubkey,
    pub market_event_queue: Pubkey,
    pub max_coin_amount: u64,
    pub max_pc_amount: u64,
    pub another_min_amount: Option<u64>,
    pub base_side: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmmWithdrawInfoResult {
    pub pool_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub market_coin_vault: Pubkey,
    pub market_pc_vault: Pubkey,
    pub market_vault_signer: Pubkey,
    pub market_event_queue: Pubkey,
    pub market_bids: Pubkey,
    pub market_asks: Pubkey,
    pub receive_min_coin_amount: Option<u64>,
    pub receive_min_pc_amount: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmmSwapInfoResult {
    pub pool_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub market_coin_vault: Pubkey,
    pub market_pc_vault: Pubkey,
    pub market_vault_signer: Pubkey,
    pub market_event_queue: Pubkey,
    pub market_bids: Pubkey,
    pub market_asks: Pubkey,
    pub amount_specified: u64,
    pub other_amount_threshold: u64,
}
