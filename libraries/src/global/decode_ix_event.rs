use anchor_client::ClientError;
use anyhow::Result;
use colorful::Color;
use colorful::Colorful;
use solana_transaction_status::option_serializer::OptionSerializer;

use crate::amm;
use crate::clmm;
use crate::common;
use crate::cpswap;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

pub fn parse_program_instruction(
    tx_id: String,
    config: &common::types::CommonConfig,
) -> Result<(), ClientError> {
    let rpc_client = RpcClient::new(config.cluster().url());
    let signature = Signature::from_str(&tx_id).unwrap();
    let tx = rpc_client.get_transaction_with_config(
        &signature,
        RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    )?;
    let transaction = tx.transaction;
    // get meta
    let meta = if transaction.meta.is_some() {
        transaction.meta
    } else {
        None
    };
    // get encoded_transaction
    let encoded_transaction = transaction.transaction;
    // // decode instruction data
    let ui_raw_msg = match encoded_transaction {
        solana_transaction_status::EncodedTransaction::Json(ui_tx) => {
            let ui_message = ui_tx.message;
            // println!("{:#?}", ui_message);
            match ui_message {
                solana_transaction_status::UiMessage::Raw(ui_raw_msg) => ui_raw_msg,
                _ => solana_transaction_status::UiRawMessage {
                    header: solana_sdk::message::MessageHeader::default(),
                    account_keys: Vec::new(),
                    recent_blockhash: "".to_string(),
                    instructions: Vec::new(),
                    address_table_lookups: None,
                },
            }
        }
        _ => solana_transaction_status::UiRawMessage {
            header: solana_sdk::message::MessageHeader::default(),
            account_keys: Vec::new(),
            recent_blockhash: "".to_string(),
            instructions: Vec::new(),
            address_table_lookups: None,
        },
    };
    // append lookup table keys if necessary
    if meta.is_some() {
        let mut account_keys = ui_raw_msg.account_keys;
        let meta = meta.clone().unwrap();
        match meta.loaded_addresses {
            OptionSerializer::Some(addresses) => {
                let mut writeable_address = addresses.writable;
                let mut readonly_address = addresses.readonly;
                account_keys.append(&mut writeable_address);
                account_keys.append(&mut readonly_address);
            }
            _ => {}
        }
        let clmm_program_index = account_keys
            .iter()
            .position(|r| r == &config.clmm_program().to_string());
        let cp_program_index = account_keys
            .iter()
            .position(|r| r == &config.cp_program().to_string());
        let amm_program_index = account_keys
            .iter()
            .position(|r| r == &config.amm_program().to_string());
        // println!("{}", program_index);
        // println!("{:#?}", account_keys);
        for (i, ui_compiled_instruction) in ui_raw_msg.instructions.iter().enumerate() {
            if let Some(program_index) = clmm_program_index {
                if (ui_compiled_instruction.program_id_index as usize) == program_index {
                    let out_put = format!("clmm instruction #{}", i + 1);
                    println!("{}", out_put.gradient(Color::Green));
                    clmm::decode_ix_event::handle_program_instruction(
                        &ui_compiled_instruction.data,
                        common::InstructionDecodeType::Base58,
                    )?;
                }
            }
            if let Some(program_index) = cp_program_index {
                if (ui_compiled_instruction.program_id_index as usize) == program_index {
                    let out_put = format!("cpswap instruction #{}", i + 1);
                    println!("{}", out_put.gradient(Color::Green));
                    cpswap::decode_ix_event::handle_program_instruction(
                        &ui_compiled_instruction.data,
                        common::InstructionDecodeType::Base58,
                    )?;
                }
            }
            if let Some(program_index) = amm_program_index {
                if (ui_compiled_instruction.program_id_index as usize) == program_index {
                    let out_put = format!("amm instruction #{}", i + 1);
                    println!("{}", out_put.gradient(Color::Green));
                    amm::decode_ix_event::handle_program_instruction(
                        &ui_compiled_instruction.data,
                        common::InstructionDecodeType::Base58,
                    )?;
                }
            }
        }

        match meta.inner_instructions {
            OptionSerializer::Some(inner_instructions) => {
                for inner in inner_instructions {
                    for (i, instruction) in inner.instructions.iter().enumerate() {
                        match instruction {
                            solana_transaction_status::UiInstruction::Compiled(
                                ui_compiled_instruction,
                            ) => {
                                if let Some(program_index) = clmm_program_index {
                                    if (ui_compiled_instruction.program_id_index as usize)
                                        == program_index
                                    {
                                        let out_put = format!(
                                            "clmm inner_instruction #{}.{}",
                                            inner.index + 1,
                                            i + 1
                                        );
                                        println!("{}", out_put.gradient(Color::Green));
                                        clmm::decode_ix_event::handle_program_instruction(
                                            &ui_compiled_instruction.data,
                                            common::InstructionDecodeType::Base58,
                                        )?;
                                    }
                                }
                                if let Some(program_index) = cp_program_index {
                                    if (ui_compiled_instruction.program_id_index as usize)
                                        == program_index
                                    {
                                        let out_put = format!(
                                            "cpswap inner_instruction #{}.{}",
                                            inner.index + 1,
                                            i + 1
                                        );
                                        println!("{}", out_put.gradient(Color::Green));
                                        cpswap::decode_ix_event::handle_program_instruction(
                                            &ui_compiled_instruction.data,
                                            common::InstructionDecodeType::Base58,
                                        )?;
                                    }
                                }
                                if let Some(program_index) = amm_program_index {
                                    if (ui_compiled_instruction.program_id_index as usize)
                                        == program_index
                                    {
                                        let out_put = format!(
                                            "amm inner_instruction #{}.{}",
                                            inner.index + 1,
                                            i + 1
                                        );
                                        println!("{}", out_put.gradient(Color::Green));
                                        amm::decode_ix_event::handle_program_instruction(
                                            &ui_compiled_instruction.data,
                                            common::InstructionDecodeType::Base58,
                                        )?;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
