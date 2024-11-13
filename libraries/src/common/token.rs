use solana_sdk::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, system_instruction,
};

pub fn create_ata_token_or_not(
    funding: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    token_program: Option<&Pubkey>,
) -> Vec<Instruction> {
    vec![
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            funding,
            owner,
            mint,
            token_program.unwrap_or(&spl_token::id()),
        ),
    ]
}

pub fn create_init_token(
    token: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    funding: &Pubkey,
    lamports: u64,
) -> Vec<Instruction> {
    vec![
        solana_sdk::system_instruction::create_account(
            funding,
            token,
            lamports,
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_account(&spl_token::id(), token, mint, owner).unwrap(),
    ]
}

pub fn create_init_mint(
    funding: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Pubkey,
    decimals: u8,
    lamports: u64,
) -> Vec<Instruction> {
    vec![
        solana_sdk::system_instruction::create_account(
            funding,
            mint,
            lamports,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            mint,
            mint_authority,
            None,
            decimals,
        )
        .unwrap(),
    ]
}

pub fn mint_to(
    mint: &Pubkey,
    to_token: &Pubkey,
    mint_authority: &Pubkey,
    token_program: Option<&Pubkey>,
    amount: u64,
) -> Vec<Instruction> {
    vec![spl_token_2022::instruction::mint_to(
        token_program.unwrap_or(&spl_token::id()),
        mint,
        &to_token,
        &mint_authority,
        &[],
        amount,
    )
    .unwrap()]
}

pub fn transfer_to(
    from: &Pubkey,
    to: &Pubkey,
    from_authority: &Pubkey,
    token_program: Option<&Pubkey>,
    amount: u64,
) -> Vec<Instruction> {
    vec![spl_token::instruction::transfer(
        token_program.unwrap_or(&spl_token::id()),
        from,
        to,
        &from_authority,
        &[],
        amount,
    )
    .unwrap()]
}

pub fn close_spl_account(
    close_account: &Pubkey,
    destination: &Pubkey,
    close_authority: &Pubkey,
    token_program: Option<&Pubkey>,
) -> Vec<Instruction> {
    vec![spl_token_2022::instruction::close_account(
        token_program.unwrap_or(&spl_token::id()),
        close_account,
        destination,
        &close_authority,
        &[],
    )
    .unwrap()]
}

pub fn wrap_sol_instructions(from: &Pubkey, to: &Pubkey, amount: u64) -> Vec<Instruction> {
    vec![
        system_instruction::transfer(from, to, amount),
        spl_token::instruction::sync_native(&spl_token::id(), to).unwrap(),
    ]
}
