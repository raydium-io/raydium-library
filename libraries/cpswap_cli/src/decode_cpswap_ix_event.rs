use anchor_client::ClientError;
use anchor_lang::Discriminator;
use anyhow::Result;
use common::{common_types, InstructionDecodeType};
use raydium_cp_swap::instruction;
use raydium_cp_swap::states::*;
use anchor_lang::__private::base64::{Engine, engine::general_purpose::STANDARD};

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
            let borsh_bytes = match STANDARD.decode(instr_data) {
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
    let disc: &[u8] = &ix_data[..8];
    ix_data = &ix_data[8..];
    // println!("{:?}", disc);

    match disc {
        instruction::CreateAmmConfig::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::CreateAmmConfig>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct CreateAmmConfig {
                pub index: u16,
                pub trade_fee_rate: u64,
                pub protocol_fee_rate: u64,
                pub fund_fee_rate: u64,
                pub create_pool_fee: u64,
            }
            impl From<instruction::CreateAmmConfig> for CreateAmmConfig {
                fn from(instr: instruction::CreateAmmConfig) -> CreateAmmConfig {
                    CreateAmmConfig {
                        index: instr.index,
                        trade_fee_rate: instr.trade_fee_rate,
                        protocol_fee_rate: instr.protocol_fee_rate,
                        fund_fee_rate: instr.fund_fee_rate,
                        create_pool_fee: instr.create_pool_fee,
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
                pub value: u64,
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
        instruction::Initialize::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::Initialize>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct Initialize {
                pub init_amount_0: u64,
                pub init_amount_1: u64,
                pub open_time: u64,
            }
            impl From<instruction::Initialize> for Initialize {
                fn from(instr: instruction::Initialize) -> Initialize {
                    Initialize {
                        init_amount_0: instr.init_amount_0,
                        init_amount_1: instr.init_amount_1,
                        open_time: instr.open_time,
                    }
                }
            }
            println!("{:#?}", Initialize::from(ix));
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
        instruction::Deposit::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::Deposit>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct Deposit {
                pub lp_token_amount: u64,
                pub maximum_token_0_amount: u64,
                pub maximum_token_1_amount: u64,
            }
            impl From<instruction::Deposit> for Deposit {
                fn from(instr: instruction::Deposit) -> Deposit {
                    Deposit {
                        lp_token_amount: instr.lp_token_amount,
                        maximum_token_0_amount: instr.maximum_token_0_amount,
                        maximum_token_1_amount: instr.maximum_token_1_amount,
                    }
                }
            }
            println!("{:#?}", Deposit::from(ix));
        }
        instruction::Withdraw::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::Withdraw>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct Withdraw {
                pub lp_token_amount: u64,
                pub minimum_token_0_amount: u64,
                pub minimum_token_1_amount: u64,
            }
            impl From<instruction::Withdraw> for Withdraw {
                fn from(instr: instruction::Withdraw) -> Withdraw {
                    Withdraw {
                        lp_token_amount: instr.lp_token_amount,
                        minimum_token_0_amount: instr.minimum_token_0_amount,
                        minimum_token_1_amount: instr.minimum_token_1_amount,
                    }
                }
            }
            println!("{:#?}", Withdraw::from(ix));
        }
        instruction::SwapBaseInput::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::SwapBaseInput>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct SwapBaseInput {
                pub amount_in: u64,
                pub minimum_amount_out: u64,
            }
            impl From<instruction::SwapBaseInput> for SwapBaseInput {
                fn from(instr: instruction::SwapBaseInput) -> SwapBaseInput {
                    SwapBaseInput {
                        amount_in: instr.amount_in,
                        minimum_amount_out: instr.minimum_amount_out,
                    }
                }
            }
            println!("{:#?}", SwapBaseInput::from(ix));
        }
        instruction::SwapBaseOutput::DISCRIMINATOR => {
            let ix = decode_instruction::<instruction::SwapBaseOutput>(&mut ix_data).unwrap();
            #[derive(Debug)]
            pub struct SwapBaseOutput {
                pub max_amount_in: u64,
                pub amount_out: u64,
            }
            impl From<instruction::SwapBaseOutput> for SwapBaseOutput {
                fn from(instr: instruction::SwapBaseOutput) -> SwapBaseOutput {
                    SwapBaseOutput {
                        max_amount_in: instr.max_amount_in,
                        amount_out: instr.amount_out,
                    }
                }
            }
            println!("{:#?}", SwapBaseOutput::from(ix));
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
        let borsh_bytes = match STANDARD.decode(log) {
            Ok(borsh_bytes) => borsh_bytes,
            _ => {
                println!("Could not base64 decode log: {}", log);
                return Ok(());
            }
        };

        let mut slice: &[u8] = &borsh_bytes[..];
        let disc: &[u8] = &slice[..8];
        slice = &slice[8..];
        match disc {
            LpChangeEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<LpChangeEvent>(&mut slice)?);
            }
            SwapEvent::DISCRIMINATOR => {
                println!("{:#?}", decode_event::<SwapEvent>(&mut slice)?);
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
