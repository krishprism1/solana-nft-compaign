use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient funds for purchase")]
    InsufficientFunds,
    #[msg("Mint amount exceeded")]
    MintAmountExceed,
    #[msg("Buy period exceeded")]
    BuyPeriodExceed,
    #[msg("Reveal period exceeded")]
    RevealPeriodExceed,
}