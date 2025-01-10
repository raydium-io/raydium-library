use anchor_client::Cluster;
use anyhow::Result;
use clap::Parser;
use solana_sdk::pubkey::Pubkey;
use spl_token_2022::extension::{
    confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
    cpi_guard::CpiGuard,
    default_account_state::DefaultAccountState,
    immutable_owner::ImmutableOwner,
    interest_bearing_mint::InterestBearingConfig,
    memo_transfer::MemoTransfer,
    mint_close_authority::MintCloseAuthority,
    non_transferable::{NonTransferable, NonTransferableAccount},
    permanent_delegate::PermanentDelegate,
    transfer_fee::{TransferFeeAmount, TransferFeeConfig},
};
use std::{convert::TryInto, str::FromStr};
use toml::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenInfo {
    pub key: Pubkey,
    pub mint: Pubkey,
    pub program: Pubkey,
    pub amount: u64,
    pub decimals: u8,
}

#[derive(Debug)]
pub enum ExtensionStruct {
    ConfidentialTransferAccount(ConfidentialTransferAccount),
    ConfidentialTransferMint(ConfidentialTransferMint),
    CpiGuard(CpiGuard),
    DefaultAccountState(DefaultAccountState),
    ImmutableOwner(ImmutableOwner),
    InterestBearingConfig(InterestBearingConfig),
    MemoTransfer(MemoTransfer),
    MintCloseAuthority(MintCloseAuthority),
    NonTransferable(NonTransferable),
    NonTransferableAccount(NonTransferableAccount),
    PermanentDelegate(PermanentDelegate),
    TransferFeeConfig(TransferFeeConfig),
    TransferFeeAmount(TransferFeeAmount),
}

pub const TEN_THOUSAND: u128 = 10000;
#[derive(Debug)]
pub struct TransferFeeInfo {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub transfer_fee: u64,
}

pub enum InstructionDecodeType {
    BaseHex,
    Base64,
    Base58,
}
pub const PROGRAM_LOG: &str = "Program log: ";
pub const PROGRAM_DATA: &str = "Program data: ";
pub const RAY_LOG: &str = "ray_log: ";

#[derive(Clone, Debug, Parser)]
pub struct CommonConfig {
    #[clap(global = true, long = "config.http")]
    http_url: Option<String>,
    #[clap(global = true, long = "config.ws")]
    ws_url: Option<String>,
    #[clap(global = true, long = "config.wallet")]
    wallet_path: Option<String>,
    #[clap(global = true, long = "config.clmm_program")]
    raydium_clmm_program: Option<Pubkey>,
    #[clap(global = true, long = "config.cp_program")]
    raydium_cp_swap_program: Option<Pubkey>,
    #[clap(global = true, long = "config.amm_program")]
    raydium_amm_program: Option<Pubkey>,
    #[clap(global = true, long = "config.openbook_program")]
    openbook_program: Option<Pubkey>,
    #[clap(global = true, long = "config.slippage")]
    slippage_bps: Option<u64>,
    #[clap(global = true, short, long, action)]
    simulate: bool,
}

impl Default for CommonConfig {
    #[cfg(not(feature = "devnet"))]
    fn default() -> Self {
        CommonConfig {
            http_url: Some("https://api.mainnet-beta.solana.com".to_string()),
            ws_url: Some("wss://api.mainnet-beta.solana.com".to_string()),
            // Default is empty.
            // Must be specified by user, as setting a default wallet may be dangerous.
            wallet_path: Some("".to_string()),
            raydium_clmm_program: Some(
                Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap(),
            ),
            raydium_cp_swap_program: Some(
                Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap(),
            ),
            raydium_amm_program: Some(
                Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap(),
            ),
            openbook_program: Some(
                Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX").unwrap(),
            ),
            slippage_bps: Some(100),
            simulate: false,
        }
    }
    #[cfg(feature = "devnet")]
    fn default() -> Self {
        CommonConfig {
            http_url: Some("https://api.devnet.solana.com".to_string()),
            ws_url: Some("wss://api.devnet.solana.com".to_string()),
            // Default is empty.
            // Must be specified by user, as setting a default wallet may be dangerous.
            wallet_path: Some("".to_string()),
            raydium_clmm_program: Some(
                Pubkey::from_str("devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH").unwrap(),
            ),
            raydium_cp_swap_program: Some(
                Pubkey::from_str("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW").unwrap(),
            ),
            raydium_amm_program: Some(
                Pubkey::from_str("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8").unwrap(),
            ),
            openbook_program: Some(
                Pubkey::from_str("EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj").unwrap(),
            ),
            slippage_bps: Some(100),
            simulate: false,
        }
    }
}

impl CommonConfig {
    pub fn file_override(&mut self) -> Result<()> {
        let mut current_dir = std::env::current_dir().unwrap();
        current_dir.push("Raydium.toml");
        if !current_dir.exists() {
            // config file not exist
            return Ok(());
        }
        // read config file
        let config_file = std::fs::read_to_string(current_dir).unwrap();

        // parse config file content
        let config_file_value: toml::Value = toml::from_str(&config_file).unwrap();
        // parse config file
        if let Some(cluster) = config_file_value.get("cluster") {
            if let Some(http_url) = cluster.get("http_url").and_then(Value::as_str) {
                if !http_url.is_empty() {
                    self.http_url = Some(http_url.to_string());
                }
            }
            if let Some(ws_url) = cluster.get("ws_url").and_then(Value::as_str) {
                if !ws_url.is_empty() {
                    self.ws_url = Some(ws_url.to_string());
                }
            }
        }
        if let Some(program) = config_file_value.get("program") {
            if let Some(raydium_clmm_program) =
                program.get("raydium_clmm_program").and_then(Value::as_str)
            {
                if !raydium_clmm_program.is_empty() {
                    self.raydium_clmm_program =
                        Some(Pubkey::from_str(raydium_clmm_program).unwrap());
                }
            }
            if let Some(raydium_cp_swap_program) = program
                .get("raydium_cp_swap_program")
                .and_then(Value::as_str)
            {
                if !raydium_cp_swap_program.is_empty() {
                    self.raydium_cp_swap_program =
                        Some(Pubkey::from_str(raydium_cp_swap_program).unwrap());
                }
            }
            if let Some(raydium_amm_program) =
                program.get("raydium_amm_program").and_then(Value::as_str)
            {
                if !raydium_amm_program.is_empty() {
                    self.raydium_amm_program = Some(Pubkey::from_str(raydium_amm_program).unwrap());
                }
            }
            if let Some(openbook_program) = program.get("openbook_program").and_then(Value::as_str)
            {
                if !openbook_program.is_empty() {
                    self.openbook_program = Some(Pubkey::from_str(openbook_program).unwrap());
                }
            }
        }
        if let Some(info) = config_file_value.get("info") {
            if let Some(wallet_path) = info.get("wallet_path").and_then(Value::as_str) {
                if !wallet_path.is_empty() {
                    self.wallet_path = Some(wallet_path.to_string());
                }
            }
            if let Some(slippage_bps) = info.get("slippage_bps").and_then(Value::as_integer) {
                self.slippage_bps = Some(slippage_bps.try_into().unwrap());
            }
        }
        return Ok(());
    }

    pub fn command_override(&mut self, command: CommonConfig) {
        if command.http_url.is_some() {
            self.http_url = command.http_url;
        }
        if command.ws_url.is_some() {
            self.ws_url = command.ws_url;
        }
        if command.wallet_path.is_some() {
            self.wallet_path = command.wallet_path;
        }
        if command.raydium_clmm_program.is_some() {
            self.raydium_clmm_program = command.raydium_clmm_program;
        }
        if command.raydium_cp_swap_program.is_some() {
            self.raydium_cp_swap_program = command.raydium_cp_swap_program;
        }
        if command.raydium_amm_program.is_some() {
            self.raydium_amm_program = command.raydium_amm_program;
        }
        if command.openbook_program.is_some() {
            self.openbook_program = command.openbook_program;
        }
        if command.slippage_bps.is_some() {
            self.slippage_bps = command.slippage_bps;
        }
        self.simulate = command.simulate;
    }

    pub fn cluster(&self) -> Cluster {
        let http_url = self.clone().http_url.unwrap_or("".to_string());
        let ws_url = self.clone().ws_url.unwrap_or("".to_string());
        Cluster::Custom(http_url, ws_url)
    }

    pub fn set_cluster(&mut self, http_url: &str, ws_url: &str) {
        self.http_url = Some(http_url.to_string());
        self.ws_url = Some(ws_url.to_string());
    }

    pub fn wallet(&self) -> String {
        self.clone().wallet_path.unwrap_or("".to_string())
    }

    pub fn set_wallet(&mut self, wallet_path: &str) {
        self.wallet_path = Some(wallet_path.to_string());
    }

    pub fn clmm_program(&self) -> Pubkey {
        if self.raydium_clmm_program.is_none() {
            Pubkey::default()
        } else {
            self.raydium_clmm_program.unwrap()
        }
    }

    pub fn set_clmm_program(&mut self, clmm_program: &str) {
        self.raydium_clmm_program = Some(Pubkey::from_str(clmm_program).unwrap());
    }

    pub fn cp_program(&self) -> Pubkey {
        if self.raydium_cp_swap_program.is_none() {
            Pubkey::default()
        } else {
            self.raydium_cp_swap_program.unwrap()
        }
    }

    pub fn set_cp_program(&mut self, cp_swap_program: &str) {
        self.raydium_cp_swap_program = Some(Pubkey::from_str(cp_swap_program).unwrap());
    }

    pub fn amm_program(&self) -> Pubkey {
        if self.raydium_amm_program.is_none() {
            Pubkey::default()
        } else {
            self.raydium_amm_program.unwrap()
        }
    }

    pub fn set_amm_program(&mut self, amm_program: &str) {
        self.raydium_amm_program = Some(Pubkey::from_str(amm_program).unwrap());
    }

    pub fn openbook_program(&self) -> Pubkey {
        if self.openbook_program.is_none() {
            Pubkey::default()
        } else {
            self.openbook_program.unwrap()
        }
    }
    pub fn set_openbook_program(&mut self, openbook_program: &str) {
        self.openbook_program = Some(Pubkey::from_str(openbook_program).unwrap());
    }

    pub fn slippage(&self) -> u64 {
        self.slippage_bps.unwrap_or(0)
    }

    pub fn set_slippage(&mut self, slippage_bps: u64) {
        self.slippage_bps = Some(slippage_bps);
    }

    pub fn simulate(&self) -> bool {
        self.simulate
    }

    pub fn set_simulate(&mut self, simulate: bool) {
        self.simulate = simulate;
    }
}
