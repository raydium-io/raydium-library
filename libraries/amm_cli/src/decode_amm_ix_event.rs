use anchor_client::ClientError;
use anyhow::Result;
use common::{common_types, InstructionDecodeType};
use raydium_amm::{instruction::*, log::decode_ray_log};
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

    let ix_data: &[u8] = &data[..];
    // println!("{:?}", disc);
    let instruction = AmmInstruction::unpack(ix_data)?;
    println!("{:#?}", instruction);
    Ok(())
}

pub fn handle_program_event(log_event: &str, with_prefix: bool) -> Result<(), ClientError> {
    // Log emitted from the current program.
    if let Some(log) = if with_prefix {
        log_event
            .strip_prefix(common_types::PROGRAM_LOG)
            .or_else(|| log_event.strip_prefix(common_types::PROGRAM_DATA))
            .or_else(|| log_event.strip_prefix(common_types::RAY_LOG))
    } else {
        Some(log_event)
    } {
        decode_ray_log(log);
        return Ok(());
    } else {
        return Ok(());
    }
}
