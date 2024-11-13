use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, PartialEq)]
pub struct CpSwapLiquidityChangeResult {
    pub pool_id: Pubkey,
    pub mint0: Pubkey,
    pub mint1: Pubkey,
    pub mintlp: Pubkey,
    pub vault0: Pubkey,
    pub vault1: Pubkey,
    pub mint0_token_program: Pubkey,
    pub mint1_token_program: Pubkey,
    pub lp_token_amount: u64,
    pub amount_0: u64,
    pub amount_1: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpSwapSwapChangeResult {
    pub pool_id: Pubkey,
    pub pool_config: Pubkey,
    pub pool_observation: Pubkey,
    pub user_input_token: Pubkey,
    pub input_vault: Pubkey,
    pub output_vault: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub input_token_program: Pubkey,
    pub output_token_program: Pubkey,
    pub amount_specified: u64,
    pub other_amount_threshold: u64,
}
