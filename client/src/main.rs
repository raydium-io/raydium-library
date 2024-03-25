#![allow(dead_code)]

use anyhow::{format_err, Result};
use raydium_library::amm;
use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

fn send_init_amm_pool_tx() -> Result<()> {
    // config params
    let wallet_file_path = "id.json";
    let cluster_url = "https://api.devnet.solana.com/";
    let amm_program = Pubkey::from_str("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8")?;
    let market_program = Pubkey::from_str("EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj")?;

    let client = RpcClient::new(cluster_url.to_string());
    let wallet = solana_sdk::signature::read_keypair_file(wallet_file_path)
        .map_err(|_| format_err!("failed to read keypair from {}", wallet_file_path))?;
    let market = Pubkey::from_str("74yqm5ihhMg5XJeqvC6oPsHaczjF6U9Rc8zs4wMnAGUL")?;
    let amm_coin_mint = Pubkey::from_str("2SiSpNowr7zUv5ZJHuzHszskQNaskWsNukhivCtuVLHo")?;
    let amm_pc_mint = Pubkey::from_str("GfmdKWR1KrttDsQkJfwtXovZw9bUBHYkPAEwB6wZqQvJ")?;
    // maintnet: 7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eqaYcHQqtj2G5
    // devnet: 3XMrhbv989VxAMi3DErLV9eJht1pHppW5LbKxe9fkEFR
    let create_fee_destination = Pubkey::from_str("3XMrhbv989VxAMi3DErLV9eJht1pHppW5LbKxe9fkEFR")?;
    let input_pc_amount = 10000_000000;
    let input_coin_amount = 10000_000000;

    let amm_keys = raydium_library::amm::utils::get_amm_pda_keys(
        &amm_program,
        &market_program,
        &market,
        &amm_coin_mint,
        &amm_pc_mint,
    )?;

    let build_init_instruction = raydium_library::amm::instructions::initialize_amm_pool(
        &amm_program,
        &amm_keys,
        &create_fee_destination,
        &wallet.pubkey(),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_coin_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_pc_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_lp_mint,
        ),
        0,
        input_pc_amount,
        input_coin_amount,
    )?;
    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &vec![build_init_instruction],
        Some(&wallet.pubkey()),
        &vec![&wallet],
        client.get_latest_blockhash()?,
    );
    let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
    println!("amm_pool_id:{}", amm_keys.amm_pool);
    println!("sig:{:#?}", sig);
    Ok(())
}

fn send_deposit_amm_pool_tx() -> Result<()> {
    // config params
    let wallet_file_path = "id.json";
    let cluster_url = "https://api.devnet.solana.com/";
    let amm_program = Pubkey::from_str("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8")?;
    let amm_pool_id = Pubkey::from_str("BbZjQanvSaE9me4adAitmTTaSgASvzaVignt4HRSM7ww")?;
    let slippage_bps = 50u64; // 0.5%
    let input_amount = 10000_000000;
    let base_side = 0; // 0: base coin; 1: base pc

    let client = RpcClient::new(cluster_url.to_string());
    let wallet = solana_sdk::signature::read_keypair_file(wallet_file_path)
        .map_err(|_| format_err!("failed to read keypair from {}", wallet_file_path))?;

    // load amm keys
    let amm_keys = raydium_library::amm::utils::load_amm_keys(&client, &amm_program, &amm_pool_id)?;
    // load market keys
    let market_keys = raydium_library::amm::openbook::get_keys_for_market(
        &client,
        &amm_keys.market_program,
        &amm_keys.market,
    )?;
    // calculate amm pool vault with load data at the same time or use simulate to calculate
    let result = raydium_library::amm::calculate_pool_vault_amounts(
        &client,
        &amm_program,
        &amm_pool_id,
        &amm_keys,
        &market_keys,
        amm::utils::CalculateMethod::Simulate(wallet.pubkey()),
    )?;
    let (max_coin_amount, max_pc_amount) =
        raydium_library::amm::amm_math::deposit_amount_with_slippage(
            result.pool_pc_vault_amount,
            result.pool_coin_vault_amount,
            input_amount,
            base_side,
            slippage_bps,
        )?;
    println!("max_coin: {}, max_pc: {}", max_coin_amount, max_pc_amount);

    let build_deposit_instruction = raydium_library::amm::instructions::deposit(
        &amm_program,
        &amm_keys,
        &market_keys,
        &wallet.pubkey(),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_coin_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_pc_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_lp_mint,
        ),
        max_coin_amount,
        max_pc_amount,
        base_side,
    )?;

    // send deposit tx
    let txn = Transaction::new_signed_with_payer(
        &vec![build_deposit_instruction],
        Some(&wallet.pubkey()),
        &vec![&wallet],
        client.get_latest_blockhash()?,
    );
    let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
    println!("sig:{:#?}", sig);
    Ok(())
}

fn send_withdraw_amm_pool_tx() -> Result<()> {
    // config params
    let wallet_file_path = "id.json";
    let cluster_url = "https://api.devnet.solana.com/";
    let amm_program = Pubkey::from_str("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8")?;
    let amm_pool_id = Pubkey::from_str("BbZjQanvSaE9me4adAitmTTaSgASvzaVignt4HRSM7ww")?;
    // let slippage_bps = 50u64; // 0.5%
    let withdraw_lp_amount = 10000_000000;

    let client = RpcClient::new(cluster_url.to_string());
    let wallet = solana_sdk::signature::read_keypair_file(wallet_file_path)
        .map_err(|_| format_err!("failed to read keypair from {}", wallet_file_path))?;

    // load amm keys
    let amm_keys = raydium_library::amm::utils::load_amm_keys(&client, &amm_program, &amm_pool_id)?;
    // load market keys
    let market_keys = raydium_library::amm::openbook::get_keys_for_market(
        &client,
        &amm_keys.market_program,
        &amm_keys.market,
    )?;

    let build_withdraw_instruction = raydium_library::amm::instructions::withdraw(
        &amm_program,
        &amm_keys,
        &market_keys,
        &wallet.pubkey(),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_coin_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_pc_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &amm_keys.amm_lp_mint,
        ),
        withdraw_lp_amount,
    )?;

    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &vec![build_withdraw_instruction],
        Some(&wallet.pubkey()),
        &vec![&wallet],
        client.get_latest_blockhash()?,
    );
    let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
    println!("sig:{:#?}", sig);
    Ok(())
}

fn send_swap_tx() -> Result<()> {
    // config params
    let wallet_file_path = "id.json";
    let cluster_url = "https://api.devnet.solana.com/";
    let amm_program = Pubkey::from_str("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8")?;
    let amm_pool_id = Pubkey::from_str("BbZjQanvSaE9me4adAitmTTaSgASvzaVignt4HRSM7ww")?;
    let input_token_mint = Pubkey::from_str("GfmdKWR1KrttDsQkJfwtXovZw9bUBHYkPAEwB6wZqQvJ")?;
    let output_token_mint = Pubkey::from_str("2SiSpNowr7zUv5ZJHuzHszskQNaskWsNukhivCtuVLHo")?;
    let slippage_bps = 50u64; // 0.5%
    let amount_specified = 2000_000000u64;
    let swap_base_in = false;

    let client = RpcClient::new(cluster_url.to_string());
    let wallet = solana_sdk::signature::read_keypair_file(wallet_file_path)
        .map_err(|_| format_err!("failed to read keypair from {}", wallet_file_path))?;

    // load amm keys
    let amm_keys = raydium_library::amm::utils::load_amm_keys(&client, &amm_program, &amm_pool_id)?;
    // load market keys
    let market_keys = raydium_library::amm::openbook::get_keys_for_market(
        &client,
        &amm_keys.market_program,
        &amm_keys.market,
    )?;
    // calculate amm pool vault with load data at the same time or use simulate to calculate
    let result = raydium_library::amm::calculate_pool_vault_amounts(
        &client,
        &amm_program,
        &amm_pool_id,
        &amm_keys,
        &market_keys,
        amm::utils::CalculateMethod::Simulate(wallet.pubkey()),
    )?;
    let direction = if input_token_mint == amm_keys.amm_coin_mint
        && output_token_mint == amm_keys.amm_pc_mint
    {
        amm::utils::SwapDirection::Coin2PC
    } else {
        amm::utils::SwapDirection::PC2Coin
    };
    let other_amount_threshold = raydium_library::amm::swap_with_slippage(
        result.pool_pc_vault_amount,
        result.pool_coin_vault_amount,
        result.swap_fee_numerator,
        result.swap_fee_denominator,
        direction,
        amount_specified,
        swap_base_in,
        slippage_bps,
    )?;
    println!(
        "amount_specified:{}, other_amount_threshold:{}",
        amount_specified, other_amount_threshold
    );

    let build_swap_instruction = raydium_library::amm::instructions::swap(
        &amm_program,
        &amm_keys,
        &market_keys,
        &wallet.pubkey(),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &input_token_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &output_token_mint,
        ),
        amount_specified,
        other_amount_threshold,
        swap_base_in,
    )?;

    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &vec![build_swap_instruction],
        Some(&wallet.pubkey()),
        &vec![&wallet],
        client.get_latest_blockhash()?,
    );
    let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
    println!("sig:{:#?}", sig);
    Ok(())
}

fn main() -> Result<()> {
    // send_init_amm_pool_tx()?;
    // send_deposit_amm_pool_tx()?;
    // send_withdraw_amm_pool_tx()?;
    // send_swap_tx()?;
    Ok(())
}
