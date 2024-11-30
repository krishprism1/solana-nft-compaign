use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid time periods provided.")]
    InvalidTimePeriods,
    #[msg("Purchase period has not started or has ended.")]
    NotInPurchasePeriod,
    #[msg("NFT limit reached.")]
    NftLimitReached,
    #[msg("Insufficient funds for purchase")]
    InsufficientFunds,
    #[msg("Mint amount exceeded")]
    MintAmountExceed,
    #[msg("Buy period exceeded")]
    BuyPeriodExceed,
    #[msg("Reveal period has not started or has ended.")]
    NotInRevealPeriod,
    #[msg("NFT not found.")]
    NftNotFound,
    #[msg("NFT has already been revealed.")]
    NftAlreadyRevealed, 
    #[msg("No available numbers to assign.")]
    NoAvailableNumbers,
    #[msg("Reveal period exceeded")]
    RevealPeriodExceed,
}