use crate::common_types::{ExtensionStruct, TokenInfo, TransferFeeInfo, TEN_THOUSAND};
use anchor_lang::AccountDeserialize;
use anyhow::{format_err, Result};
use solana_account_decoder::{
    parse_token::{TokenAccountType, UiAccountState},
    UiAccountData,
};
use solana_client::{rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{account::Account as CliAccount, pubkey::Pubkey, signer::keypair::Keypair};
use anchor_lang::solana_program::program_pack::Pack;
use spl_token_2022::{
    extension::{
        confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
        cpi_guard::CpiGuard,
        default_account_state::DefaultAccountState,
        immutable_owner::ImmutableOwner,
        interest_bearing_mint::InterestBearingConfig,
        memo_transfer::MemoTransfer,
        mint_close_authority::MintCloseAuthority,
        non_transferable::{NonTransferable, NonTransferableAccount},
        permanent_delegate::PermanentDelegate,
        transfer_fee::{TransferFeeAmount, TransferFeeConfig, MAX_FEE_BASIS_POINTS},
        BaseState, BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    state::{Account, Mint},
};
use std::convert::TryFrom;

pub fn amount_with_slippage(amount: u64, slippage_bps: u64, up_towards: bool) -> Result<u64> {
    let amount = amount as u128;
    let slippage_bps = slippage_bps as u128;
    let amount_with_slippage = if up_towards {
        amount
            .checked_mul(slippage_bps.checked_add(TEN_THOUSAND).unwrap())
            .unwrap()
            .checked_div(TEN_THOUSAND)
            .unwrap()
    } else {
        amount
            .checked_mul(TEN_THOUSAND.checked_sub(slippage_bps).unwrap())
            .unwrap()
            .checked_div(TEN_THOUSAND)
            .unwrap()
    };
    u64::try_from(amount_with_slippage)
        .map_err(|_| format_err!("failed to read keypair from {}", amount_with_slippage))
}

pub fn read_keypair_file(s: &str) -> Result<Keypair> {
    solana_sdk::signature::read_keypair_file(s)
        .map_err(|_| format_err!("failed to read keypair from {}", s))
}

pub fn unpack_token(token_data: &[u8]) -> Result<StateWithExtensions<Account>> {
    let token = StateWithExtensions::<Account>::unpack(&token_data)?;
    Ok(token)
}

pub fn unpack_mint(token_data: &[u8]) -> Result<StateWithExtensions<Mint>> {
    let mint = StateWithExtensions::<Mint>::unpack(&token_data)?;
    Ok(mint)
}

pub fn deserialize_anchor_account<T: AccountDeserialize>(account: &CliAccount) -> Result<T> {
    let mut data: &[u8] = &account.data;
    T::try_deserialize(&mut data).map_err(Into::into)
}

pub fn deserialize_account<T: Copy>(account: &CliAccount, is_anchor_account: bool) -> Result<T> {
    let mut account_data = account.data.as_slice();
    if is_anchor_account {
        account_data = &account_data[8..std::mem::size_of::<T>() + 8];
    }
    Ok(unsafe { *(&account_data[0] as *const u8 as *const T) })
}

pub fn get_pool_mints_inverse_fee(
    rpc_client: &RpcClient,
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    post_fee_amount_0: u64,
    post_fee_amount_1: u64,
) -> (TransferFeeInfo, TransferFeeInfo) {
    let load_accounts = vec![token_mint_0, token_mint_1];
    let rsps = rpc_client.get_multiple_accounts(&load_accounts).unwrap();
    let epoch = rpc_client.get_epoch_info().unwrap().epoch;
    let mint0_account = rsps[0].clone().ok_or("load mint0 rps error!").unwrap();
    let mint1_account = rsps[1].clone().ok_or("load mint0 rps error!").unwrap();
    let mint0_state = unpack_mint(&mint0_account.data).unwrap();
    let mint1_state = unpack_mint(&mint1_account.data).unwrap();
    (
        TransferFeeInfo {
            mint: token_mint_0,
            owner: mint0_account.owner,
            transfer_fee: get_transfer_inverse_fee(&mint0_state, post_fee_amount_0, epoch),
        },
        TransferFeeInfo {
            mint: token_mint_1,
            owner: mint1_account.owner,
            transfer_fee: get_transfer_inverse_fee(&mint1_state, post_fee_amount_1, epoch),
        },
    )
}

pub fn get_pool_mints_transfer_fee(
    rpc_client: &RpcClient,
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    pre_fee_amount_0: u64,
    pre_fee_amount_1: u64,
) -> (TransferFeeInfo, TransferFeeInfo) {
    let load_accounts = vec![token_mint_0, token_mint_1];
    let rsps = rpc_client.get_multiple_accounts(&load_accounts).unwrap();
    let epoch = rpc_client.get_epoch_info().unwrap().epoch;
    let mint0_account = rsps[0].clone().ok_or("load mint0 rps error!").unwrap();
    let mint1_account = rsps[1].clone().ok_or("load mint0 rps error!").unwrap();
    let mint0_state = unpack_mint(&mint0_account.data).unwrap();
    let mint1_state = unpack_mint(&mint1_account.data).unwrap();
    (
        TransferFeeInfo {
            mint: token_mint_0,
            owner: mint0_account.owner,
            transfer_fee: get_transfer_fee(&mint0_state, epoch, pre_fee_amount_0),
        },
        TransferFeeInfo {
            mint: token_mint_1,
            owner: mint1_account.owner,
            transfer_fee: get_transfer_fee(&mint1_state, epoch, pre_fee_amount_1),
        },
    )
}

/// Calculate the fee for output amount
pub fn get_transfer_inverse_fee<'data, S: BaseState + Pack>(
    account_state: &StateWithExtensions<'data, S>,
    epoch: u64,
    post_fee_amount: u64,
) -> u64 {
    let fee = if let Ok(transfer_fee_config) = account_state.get_extension::<TransferFeeConfig>() {
        let transfer_fee = transfer_fee_config.get_epoch_fee(epoch);
        if u16::from(transfer_fee.transfer_fee_basis_points) == MAX_FEE_BASIS_POINTS {
            u64::from(transfer_fee.maximum_fee)
        } else {
            transfer_fee_config
                .calculate_inverse_epoch_fee(epoch, post_fee_amount)
                .unwrap()
        }
    } else {
        0
    };
    fee
}

/// Calculate the fee for input amount
pub fn get_transfer_fee<'data, S: BaseState + Pack>(
    account_state: &StateWithExtensions<'data, S>,
    epoch: u64,
    pre_fee_amount: u64,
) -> u64 {
    let fee = if let Ok(transfer_fee_config) = account_state.get_extension::<TransferFeeConfig>() {
        transfer_fee_config
            .calculate_epoch_fee(epoch, pre_fee_amount)
            .unwrap()
    } else {
        0
    };
    fee
}

pub fn get_nft_accounts_by_owner_with_specified_program(
    client: &RpcClient,
    owner: &Pubkey,
    token_program: Pubkey,
) -> Vec<TokenInfo> {
    let all_tokens = client
        .get_token_accounts_by_owner(owner, TokenAccountsFilter::ProgramId(spl_token::id()))
        .unwrap();
    let mut nft_accounts_info = Vec::new();
    for keyed_account in all_tokens {
        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if parsed_account.program == "spl-token" || parsed_account.program == "spl-token-2022" {
                if let Ok(TokenAccountType::Account(ui_token_account)) =
                    serde_json::from_value(parsed_account.parsed)
                {
                    let _frozen = ui_token_account.state == UiAccountState::Frozen;

                    let token = ui_token_account
                        .mint
                        .parse::<Pubkey>()
                        .unwrap_or_else(|err| panic!("Invalid mint: {}", err));
                    let token_account = keyed_account
                        .pubkey
                        .parse::<Pubkey>()
                        .unwrap_or_else(|err| panic!("Invalid token account: {}", err));
                    let token_amount = ui_token_account
                        .token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or_else(|err| panic!("Invalid token amount: {}", err));

                    let _close_authority = ui_token_account.close_authority.map_or(*owner, |s| {
                        s.parse::<Pubkey>()
                            .unwrap_or_else(|err| panic!("Invalid close authority: {}", err))
                    });

                    if ui_token_account.token_amount.decimals == 0 && token_amount == 1 {
                        nft_accounts_info.push(TokenInfo {
                            key: token_account,
                            mint: token,
                            program: token_program,
                            amount: token_amount,
                            decimals: ui_token_account.token_amount.decimals,
                        });
                    }
                }
            }
        }
    }
    nft_accounts_info
}

pub fn get_account_extensions<'data, S: BaseState + Pack>(
    account_state: &StateWithExtensions<'data, S>,
) -> Vec<ExtensionStruct> {
    let mut extensions: Vec<ExtensionStruct> = Vec::new();
    let extension_types = account_state.get_extension_types().unwrap();
    println!("extension_types:{:?}", extension_types);
    for extension_type in extension_types {
        match extension_type {
            ExtensionType::ConfidentialTransferAccount => {
                let extension = account_state
                    .get_extension::<ConfidentialTransferAccount>()
                    .unwrap();
                extensions.push(ExtensionStruct::ConfidentialTransferAccount(*extension));
            }
            ExtensionType::ConfidentialTransferMint => {
                let extension = account_state
                    .get_extension::<ConfidentialTransferMint>()
                    .unwrap();
                extensions.push(ExtensionStruct::ConfidentialTransferMint(*extension));
            }
            ExtensionType::CpiGuard => {
                let extension = account_state.get_extension::<CpiGuard>().unwrap();
                extensions.push(ExtensionStruct::CpiGuard(*extension));
            }
            ExtensionType::DefaultAccountState => {
                let extension = account_state
                    .get_extension::<DefaultAccountState>()
                    .unwrap();
                extensions.push(ExtensionStruct::DefaultAccountState(*extension));
            }
            ExtensionType::ImmutableOwner => {
                let extension = account_state.get_extension::<ImmutableOwner>().unwrap();
                extensions.push(ExtensionStruct::ImmutableOwner(*extension));
            }
            ExtensionType::InterestBearingConfig => {
                let extension = account_state
                    .get_extension::<InterestBearingConfig>()
                    .unwrap();
                extensions.push(ExtensionStruct::InterestBearingConfig(*extension));
            }
            ExtensionType::MemoTransfer => {
                let extension = account_state.get_extension::<MemoTransfer>().unwrap();
                extensions.push(ExtensionStruct::MemoTransfer(*extension));
            }
            ExtensionType::MintCloseAuthority => {
                let extension = account_state.get_extension::<MintCloseAuthority>().unwrap();
                extensions.push(ExtensionStruct::MintCloseAuthority(*extension));
            }
            ExtensionType::NonTransferable => {
                let extension = account_state.get_extension::<NonTransferable>().unwrap();
                extensions.push(ExtensionStruct::NonTransferable(*extension));
            }
            ExtensionType::NonTransferableAccount => {
                let extension = account_state
                    .get_extension::<NonTransferableAccount>()
                    .unwrap();
                extensions.push(ExtensionStruct::NonTransferableAccount(*extension));
            }
            ExtensionType::PermanentDelegate => {
                let extension = account_state.get_extension::<PermanentDelegate>().unwrap();
                extensions.push(ExtensionStruct::PermanentDelegate(*extension));
            }
            ExtensionType::TransferFeeConfig => {
                let extension = account_state.get_extension::<TransferFeeConfig>().unwrap();
                extensions.push(ExtensionStruct::TransferFeeConfig(*extension));
            }
            ExtensionType::TransferFeeAmount => {
                let extension = account_state.get_extension::<TransferFeeAmount>().unwrap();
                extensions.push(ExtensionStruct::TransferFeeAmount(*extension));
            }
            _ => {
                println!("unkonwn extension:{:#?}", extension_type);
            }
        }
    }
    extensions
}
