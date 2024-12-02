use anchor_client::ClientError;
use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use anyhow::Result;
use common::{common_types, InstructionDecodeType};
use raydium_amm_v3::instruction;
use raydium_amm_v3::instructions::*;
use raydium_amm_v3::states::*;

pub fn handle_program_instruction(
    instr_data: &str,
    decode_type: InstructionDecodeType,
) -> Result<(), ClientError> {
    let data;
    match decode_type {
        InstructionDecodeType::BaseHex => {
            data = hex::decode(instr_data).unwrap();
        }
        InstructionDecodeType::Base64 => {
            let borsh_bytes = match anchor_lang::__private::base64::decode(instr_data) {
                Ok(borsh_bytes) => borsh_bytes,
                _ => {
                    println!("Could not base64 decode instruction: {}", instr_data);
                    return Ok(());
                }
            };
            data = borsh_bytes;
        }
        InstructionDecodeType::Base58 => {
            let borsh_bytes = match bs58::decode(instr_data).into_vec() {
                Ok(borsh_bytes) => borsh_bytes,
                _ => {
                    println!("Could not base58 decode instruction: {}", instr_data);
                    return Ok(());
                }
            };
            data = borsh_bytes;
        }
    }

    let mut ix_data: &[u8] = &data[..];
    let disc: [u8; 8] = {
        let mut disc = [0; 8];
        disc.copy_from_slice(&data[..8]);
        ix_data = &ix_data[8..];
        disc
    };
    // println!("{:?}", disc);

    match disc {
        instruction::CreateAmmConfig::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::CreateAmmConfig>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CreateAmmConfig {
                pub index: u16,
                pub tick_spacing: u16,
                pub trade_fee_rate: u32,
                pub protocol_fee_rate: u32,
                pub fund_fee_rate: u32,
            }
            impl From<instruction::CreateAmmConfig> for CreateAmmConfig {
                fn from(instr: instruction::CreateAmmConfig) -> CreateAmmConfig {
                    CreateAmmConfig {
                        index: instr.index,
                        tick_spacing: instr.tick_spacing,
                        trade_fee_rate: instr.trade_fee_rate,
                        protocol_fee_rate: instr.protocol_fee_rate,
                        fund_fee_rate: instr.fund_fee_rate,
                    }
                }
            }
            println!("{:#?}", CreateAmmConfig::from(ix));
        }
        instruction::UpdateAmmConfig::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::UpdateAmmConfig>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct UpdateAmmConfig {
                pub param: u8,
                pub value: u32,
            }
            impl From<instruction::UpdateAmmConfig> for UpdateAmmConfig {
                fn from(instr: instruction::UpdateAmmConfig) -> UpdateAmmConfig {
                    UpdateAmmConfig {
                        param: instr.param,
                        value: instr.value,
                    }
                }
            }
            println!("{:#?}", UpdateAmmConfig::from(ix));
        }
        instruction::CreatePool::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::CreatePool>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CreatePool {
                pub sqrt_price_x64: u128,
                pub open_time: u64,
            }
            impl From<instruction::CreatePool> for CreatePool {
                fn from(instr: instruction::CreatePool) -> CreatePool {
                    CreatePool {
                        sqrt_price_x64: instr.sqrt_price_x64,
                        open_time: instr.open_time,
                    }
                }
            }
            println!("{:#?}", CreatePool::from(ix));
        }
        instruction::UpdatePoolStatus::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::UpdatePoolStatus>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct UpdatePoolStatus {
                pub status: u8,
            }
            impl From<instruction::UpdatePoolStatus> for UpdatePoolStatus {
                fn from(instr: instruction::UpdatePoolStatus) -> UpdatePoolStatus {
                    UpdatePoolStatus {
                        status: instr.status,
                    }
                }
            }
            println!("{:#?}", UpdatePoolStatus::from(ix));
        }
        instruction::CreateOperationAccount::DISCRIMINATOR => {
            let ix =
                decode_instruction::<instruction::CreateOperationAccount>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CreateOperationAccount;
            impl From<instruction::CreateOperationAccount> for CreateOperationAccount {
                fn from(_instr: instruction::CreateOperationAccount) -> CreateOperationAccount {
                    CreateOperationAccount
                }
            }
            println!("{:#?}", CreateOperationAccount::from(ix));
        }
        instruction::UpdateOperationAccount::DISCRIMINATOR => {
            let ix =
                decode_instruction::<instruction::UpdateOperationAccount>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct UpdateOperationAccount {
                pub param: u8,
                pub keys: Vec<Pubkey>,
            }
            impl From<instruction::UpdateOperationAccount> for UpdateOperationAccount {
                fn from(instr: instruction::UpdateOperationAccount) -> UpdateOperationAccount {
                    UpdateOperationAccount {
                        param: instr.param,
                        keys: instr.keys,
                    }
                }
            }
            println!("{:#?}", UpdateOperationAccount::from(ix));
        }
        instruction::TransferRewardOwner::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::TransferRewardOwner>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct TransferRewardOwner {
                pub new_owner: Pubkey,
            }
            impl From<instruction::TransferRewardOwner> for TransferRewardOwner {
                fn from(instr: instruction::TransferRewardOwner) -> TransferRewardOwner {
                    TransferRewardOwner {
                        new_owner: instr.new_owner,
                    }
                }
            }
            println!("{:#?}", TransferRewardOwner::from(ix));
        }
        instruction::InitializeReward::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::InitializeReward>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct InitializeReward {
                pub param: InitializeRewardParam,
            }
            impl From<instruction::InitializeReward> for InitializeReward {
                fn from(instr: instruction::InitializeReward) -> InitializeReward {
                    InitializeReward { param: instr.param }
                }
            }
            println!("{:#?}", InitializeReward::from(ix));
        }
        instruction::CollectRemainingRewards::DISCRIMINATOR => {
            let ix =
                decode_instruction::<instruction::CollectRemainingRewards>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CollectRemainingRewards {
                pub reward_index: u8,
            }
            impl From<instruction::CollectRemainingRewards> for CollectRemainingRewards {
                fn from(instr: instruction::CollectRemainingRewards) -> CollectRemainingRewards {
                    CollectRemainingRewards {
                        reward_index: instr.reward_index,
                    }
                }
            }
            println!("{:#?}", CollectRemainingRewards::from(ix));
        }
        instruction::UpdateRewardInfos::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::UpdateRewardInfos>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct UpdateRewardInfos;
            impl From<instruction::UpdateRewardInfos> for UpdateRewardInfos {
                fn from(_instr: instruction::UpdateRewardInfos) -> UpdateRewardInfos {
                    UpdateRewardInfos
                }
            }
            println!("{:#?}", UpdateRewardInfos::from(ix));
        }
        instruction::SetRewardParams::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::SetRewardParams>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct SetRewardParams {
                pub reward_index: u8,
                pub emissions_per_second_x64: u128,
                pub open_time: u64,
                pub end_time: u64,
            }
            impl From<instruction::SetRewardParams> for SetRewardParams {
                fn from(instr: instruction::SetRewardParams) -> SetRewardParams {
                    SetRewardParams {
                        reward_index: instr.reward_index,
                        emissions_per_second_x64: instr.emissions_per_second_x64,
                        open_time: instr.open_time,
                        end_time: instr.end_time,
                    }
                }
            }
            println!("{:#?}", SetRewardParams::from(ix));
        }
        instruction::CollectProtocolFee::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::CollectProtocolFee>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CollectProtocolFee {
                pub amount_0_requested: u64,
                pub amount_1_requested: u64,
            }
            impl From<instruction::CollectProtocolFee> for CollectProtocolFee {
                fn from(instr: instruction::CollectProtocolFee) -> CollectProtocolFee {
                    CollectProtocolFee {
                        amount_0_requested: instr.amount_0_requested,
                        amount_1_requested: instr.amount_1_requested,
                    }
                }
            }
            println!("{:#?}", CollectProtocolFee::from(ix));
        }
        instruction::CollectFundFee::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::CollectFundFee>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CollectFundFee {
                pub amount_0_requested: u64,
                pub amount_1_requested: u64,
            }
            impl From<instruction::CollectFundFee> for CollectFundFee {
                fn from(instr: instruction::CollectFundFee) -> CollectFundFee {
                    CollectFundFee {
                        amount_0_requested: instr.amount_0_requested,
                        amount_1_requested: instr.amount_1_requested,
                    }
                }
            }
            println!("{:#?}", CollectFundFee::from(ix));
        }
        instruction::OpenPosition::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::OpenPosition>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct OpenPosition {
                pub tick_lower_index: i32,
                pub tick_upper_index: i32,
                pub tick_array_lower_start_index: i32,
                pub tick_array_upper_start_index: i32,
                pub liquidity: u128,
                pub amount_0_max: u64,
                pub amount_1_max: u64,
            }
            impl From<instruction::OpenPosition> for OpenPosition {
                fn from(instr: instruction::OpenPosition) -> OpenPosition {
                    OpenPosition {
                        tick_lower_index: instr.tick_lower_index,
                        tick_upper_index: instr.tick_upper_index,
                        tick_array_lower_start_index: instr.tick_array_lower_start_index,
                        tick_array_upper_start_index: instr.tick_array_upper_start_index,
                        liquidity: instr.liquidity,
                        amount_0_max: instr.amount_0_max,
                        amount_1_max: instr.amount_1_max,
                    }
                }
            }
            println!("{:#?}", OpenPosition::from(ix));
        }
        instruction::OpenPositionV2::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::OpenPositionV2>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct OpenPositionV2 {
                pub tick_lower_index: i32,
                pub tick_upper_index: i32,
                pub tick_array_lower_start_index: i32,
                pub tick_array_upper_start_index: i32,
                pub liquidity: u128,
                pub amount_0_max: u64,
                pub amount_1_max: u64,
                pub base_flag: Option<bool>,
                pub with_metadata: bool,
            }
            impl From<instruction::OpenPositionV2> for OpenPositionV2 {
                fn from(instr: instruction::OpenPositionV2) -> OpenPositionV2 {
                    OpenPositionV2 {
                        tick_lower_index: instr.tick_lower_index,
                        tick_upper_index: instr.tick_upper_index,
                        tick_array_lower_start_index: instr.tick_array_lower_start_index,
                        tick_array_upper_start_index: instr.tick_array_upper_start_index,
                        liquidity: instr.liquidity,
                        amount_0_max: instr.amount_0_max,
                        amount_1_max: instr.amount_1_max,
                        base_flag: instr.base_flag,
                        with_metadata: instr.with_metadata,
                    }
                }
            }
            println!("{:#?}", OpenPositionV2::from(ix));
        }
        instruction::ClosePosition::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::ClosePosition>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct ClosePosition;
            impl From<instruction::ClosePosition> for ClosePosition {
                fn from(_instr: instruction::ClosePosition) -> ClosePosition {
                    ClosePosition
                }
            }
            println!("{:#?}", ClosePosition::from(ix));
        }
        instruction::IncreaseLiquidity::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::IncreaseLiquidity>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct IncreaseLiquidity {
                pub liquidity: u128,
                pub amount_0_max: u64,
                pub amount_1_max: u64,
            }
            impl From<instruction::IncreaseLiquidity> for IncreaseLiquidity {
                fn from(instr: instruction::IncreaseLiquidity) -> IncreaseLiquidity {
                    IncreaseLiquidity {
                        liquidity: instr.liquidity,
                        amount_0_max: instr.amount_0_max,
                        amount_1_max: instr.amount_1_max,
                    }
                }
            }
            println!("{:#?}", IncreaseLiquidity::from(ix));
        }
        instruction::IncreaseLiquidityV2::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::IncreaseLiquidityV2>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct IncreaseLiquidityV2 {
                pub liquidity: u128,
                pub amount_0_max: u64,
                pub amount_1_max: u64,
                pub base_flag: Option<bool>,
            }
            impl From<instruction::IncreaseLiquidityV2> for IncreaseLiquidityV2 {
                fn from(instr: instruction::IncreaseLiquidityV2) -> IncreaseLiquidityV2 {
                    IncreaseLiquidityV2 {
                        liquidity: instr.liquidity,
                        amount_0_max: instr.amount_0_max,
                        amount_1_max: instr.amount_1_max,
                        base_flag: instr.base_flag,
                    }
                }
            }
            println!("{:#?}", IncreaseLiquidityV2::from(ix));
        }
        instruction::DecreaseLiquidity::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::DecreaseLiquidity>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct DecreaseLiquidity {
                pub liquidity: u128,
                pub amount_0_min: u64,
                pub amount_1_min: u64,
            }
            impl From<instruction::DecreaseLiquidity> for DecreaseLiquidity {
                fn from(instr: instruction::DecreaseLiquidity) -> DecreaseLiquidity {
                    DecreaseLiquidity {
                        liquidity: instr.liquidity,
                        amount_0_min: instr.amount_0_min,
                        amount_1_min: instr.amount_1_min,
                    }
                }
            }
            println!("{:#?}", DecreaseLiquidity::from(ix));
        }
        instruction::DecreaseLiquidityV2::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::DecreaseLiquidityV2>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct DecreaseLiquidityV2 {
                pub liquidity: u128,
                pub amount_0_min: u64,
                pub amount_1_min: u64,
            }
            impl From<instruction::DecreaseLiquidityV2> for DecreaseLiquidityV2 {
                fn from(instr: instruction::DecreaseLiquidityV2) -> DecreaseLiquidityV2 {
                    DecreaseLiquidityV2 {
                        liquidity: instr.liquidity,
                        amount_0_min: instr.amount_0_min,
                        amount_1_min: instr.amount_1_min,
                    }
                }
            }
            println!("{:#?}", DecreaseLiquidityV2::from(ix));
        }
        instruction::Swap::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::Swap>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct Swap {
                pub amount: u64,
                pub other_amount_threshold: u64,
                pub sqrt_price_limit_x64: u128,
                pub is_base_input: bool,
            }
            impl From<instruction::Swap> for Swap {
                fn from(instr: instruction::Swap) -> Swap {
                    Swap {
                        amount: instr.amount,
                        other_amount_threshold: instr.other_amount_threshold,
                        sqrt_price_limit_x64: instr.sqrt_price_limit_x64,
                        is_base_input: instr.is_base_input,
                    }
                }
            }
            println!("{:#?}", Swap::from(ix));
        }
        instruction::SwapV2::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::SwapV2>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct SwapV2 {
                pub amount: u64,
                pub other_amount_threshold: u64,
                pub sqrt_price_limit_x64: u128,
                pub is_base_input: bool,
            }
            impl From<instruction::SwapV2> for SwapV2 {
                fn from(instr: instruction::SwapV2) -> SwapV2 {
                    SwapV2 {
                        amount: instr.amount,
                        other_amount_threshold: instr.other_amount_threshold,
                        sqrt_price_limit_x64: instr.sqrt_price_limit_x64,
                        is_base_input: instr.is_base_input,
                    }
                }
            }
            println!("{:#?}", SwapV2::from(ix));
        }
        instruction::SwapRouterBaseIn::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::SwapRouterBaseIn>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct SwapRouterBaseIn {
                pub amount_in: u64,
                pub amount_out_minimum: u64,
            }
            impl From<instruction::SwapRouterBaseIn> for SwapRouterBaseIn {
                fn from(instr: instruction::SwapRouterBaseIn) -> SwapRouterBaseIn {
                    SwapRouterBaseIn {
                        amount_in: instr.amount_in,
                        amount_out_minimum: instr.amount_out_minimum,
                    }
                }
            }
            println!("{:#?}", SwapRouterBaseIn::from(ix));
        }
        _ => {
            println!("unknow instruction: {}", instr_data);
        }
    }
    Ok(())
}

fn decode_instruction<T: anchor_lang::AnchorDeserialize>(
    slice: &mut &[u8],
) -> Result<T, anchor_lang::error::ErrorCode> {
    let instruction: T = anchor_lang::AnchorDeserialize::deserialize(slice)
        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
    Ok(instruction)
}

pub fn handle_program_event(log_event: &str, with_prefix: bool) -> Result<(), ClientError> {
    // Log emitted from the current program.
    if let Some(log) = if with_prefix {
        log_event
            .strip_prefix(common_types::PROGRAM_LOG)
            .or_else(|| log_event.strip_prefix(common_types::PROGRAM_DATA))
    } else {
        Some(log_event)
    } {
        let borsh_bytes = match anchor_lang::__private::base64::decode(log) {
            Ok(borsh_bytes) => borsh_bytes,
            _ => {
                println!("Could not base64 decode log: {}", log);
                return Ok(());
            }
        };

        let mut slice: &[u8] = &borsh_bytes[..];
        let disc: [u8; 8] = {
            let mut disc = [0; 8];
            disc.copy_from_slice(&borsh_bytes[..8]);
            slice = &slice[8..];
            disc
        };
        match disc {
            ConfigChangeEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<ConfigChangeEvent>(&mut slice)?);
            }
            CollectPersonalFeeEvent::DISCRIMINATOR => {
                println!(
                    "{:#?}",
                    decode_event::<CollectPersonalFeeEvent>(&mut slice)?
                );
            }
            CollectProtocolFeeEvent::DISCRIMINATOR => {
                println!(
                    "{:#?}",
                    decode_event::<CollectProtocolFeeEvent>(&mut slice)?
                );
            }
            CreatePersonalPositionEvent::DISCRIMINATOR => {
                println!(
                    "{:#?}",
                    decode_event::<CreatePersonalPositionEvent>(&mut slice)?
                );
            }
            DecreaseLiquidityEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<DecreaseLiquidityEvent>(&mut slice)?);
            }
            IncreaseLiquidityEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<IncreaseLiquidityEvent>(&mut slice)?);
            }
            LiquidityCalculateEvent::DISCRIMINATOR => {
                println!(
                    "{:#?}",
                    decode_event::<LiquidityCalculateEvent>(&mut slice)?
                );
            }
            LiquidityChangeEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<LiquidityChangeEvent>(&mut slice)?);
            }
            // PriceChangeEvent::DISCRIMINATOR => {
            //     println!("{:#?}", decode_event::<PriceChangeEvent>(&mut slice)?);
            // }
            SwapEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<SwapEvent>(&mut slice)?);
            }
            PoolCreatedEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<PoolCreatedEvent>(&mut slice)?);
            }
            _ => {
                println!("unknow event: {}", log_event);
            }
        }
        return Ok(());
    } else {
        return Ok(());
    }
}

fn decode_event<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(
    slice: &mut &[u8],
) -> Result<T, ClientError> {
    let event: T = anchor_lang::AnchorDeserialize::deserialize(slice)
        .map_err(|e| ClientError::LogParseError(e.to_string()))?;
    Ok(event)
}
