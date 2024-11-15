use crate::amm;
use crate::clmm;
use crate::common;
use crate::cpswap;
use crate::global;
use anyhow::Result;
use clap::Parser;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

#[derive(Debug, Parser)]
pub enum GlobalCommands {
    DecodeTx {
        // Transaction id
        #[clap(short, long)]
        tx_id: String,
    },
    DecodeIx {
        // The program of the instruction belongs to.
        // It can be amm, clmm, cpswap program's id.
        #[arg(short, long)]
        program: Pubkey,
        // Instruction hex data
        #[clap(short, long)]
        ix_data: String,
    },
    DecodeEvent {
        // The program of the instruction belongs to.
        // It can be amm, clmm, cpswap program's id.
        #[arg(short, long)]
        program: Pubkey,
        // Program event log
        #[clap(short, long)]
        event_data: String,
    },
}

pub fn process_global_commands(
    command: GlobalCommands,
    config: &common::types::CommonConfig,
) -> Result<Option<Vec<Instruction>>> {
    match command {
        GlobalCommands::DecodeTx { tx_id } => {
            global::decode_ix_event::parse_program_instruction(tx_id, config).unwrap();
            return Ok(None);
        }
        GlobalCommands::DecodeIx { program, ix_data } => {
            if program == config.clmm_program() {
                clmm::decode_ix_event::handle_program_instruction(
                    &ix_data,
                    common::InstructionDecodeType::BaseHex,
                )?;
            } else if program == config.cp_program() {
                cpswap::decode_ix_event::handle_program_instruction(
                    &ix_data,
                    common::InstructionDecodeType::BaseHex,
                )?;
            } else if program == config.amm_program() {
                amm::decode_ix_event::handle_program_instruction(
                    &ix_data,
                    common::InstructionDecodeType::BaseHex,
                )?;
            } else {
                panic!("invalid program");
            }
            return Ok(None);
        }
        GlobalCommands::DecodeEvent {
            program,
            event_data,
        } => {
            if program == config.clmm_program() {
                clmm::decode_ix_event::handle_program_event(&event_data, false)?;
            } else if program == config.cp_program() {
                cpswap::decode_ix_event::handle_program_event(&event_data, false)?;
            } else if program == config.amm_program() {
                amm::decode_ix_event::handle_program_event(&event_data, false)?;
            } else {
                panic!("invalid program");
            }
            return Ok(None);
        }
    }
}
