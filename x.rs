use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface};
use anchor_lang::solana_program::clock::Clock;
use anchor_spl::associated_token::AssociatedToken;

// This is your program's public key and it will update
// automatically when you build the project.
declare_id!("3vQXZ7xV4X8K86y97WPi1Td74CJZiFkv7qrFPuYU9yes");

#[program]
pub mod TokenVesting {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        max_nfts: u64,
        nft_price_lamports: u64,
        purchase_start: i64,
        purchase_end: i64,
        reveal_start: i64,
        reveal_end: i64,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        require!(
            purchase_start < purchase_end && reveal_start < reveal_end && purchase_end <= reveal_start,
            ErrorCode::InvalidTimePeriods
        );

        global_state.total_nfts_minted = 0;
        global_state.max_nfts = max_nfts;
        global_state.nft_price_lamports = nft_price_lamports;
        global_state.purchase_start = purchase_start;
        global_state.purchase_end = purchase_end;
        global_state.reveal_start = reveal_start;
        global_state.reveal_end = reveal_end;
        global_state.used_numbers = Vec::new();
        global_state.admin = ctx.accounts.admin.key();

        Ok(())
    }

    pub fn set_periods(
        ctx: Context<SetPeriods>,
        purchase_start: i64,
        purchase_end: i64,
        reveal_start: i64,
        reveal_end: i64,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        require!(
            purchase_start < purchase_end && reveal_start < reveal_end && purchase_end <= reveal_start,
            ErrorCode::InvalidTimePeriods
        );

        global_state.purchase_start = purchase_start;
        global_state.purchase_end = purchase_end;
        global_state.reveal_start = reveal_start;
        global_state.reveal_end = reveal_end;

        Ok(())
    }

    pub fn purchase(ctx: Context<Purchase>) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        let user_state = &mut ctx.accounts.user_state;
    
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time >= global_state.purchase_start && current_time <= global_state.purchase_end,
            ErrorCode::NotInPurchasePeriod
        );
    
        require!(
            global_state.total_nfts_minted < global_state.max_nfts,
            ErrorCode::NftLimitReached
        );
    
        // Transfer SPL tokens from the payer to the admin
        let nft_price = global_state.nft_price_lamports; // Update this variable name if using SPL token price
        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_token_account.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::transfer(
            CpiContext::new(cpi_program, cpi_accounts),
            nft_price,
        )?;
    
        // Mint NFT to the user's token account
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::mint_to(
            CpiContext::new(cpi_program, cpi_accounts),
            1, // Mint 1 NFT
        )?;
    
        // Record NFT in user's state
        user_state.nfts.push(NftInfo {
            mint: ctx.accounts.mint.key(),
            revealed_number: None,
        });
    
        // Increment total NFTs minted
        global_state.total_nfts_minted += 1;
    
        Ok(())
    }

    pub fn reveal(ctx: Context<Reveal>, mint: Pubkey) -> Result<()> {
        let user_state = &mut ctx.accounts.user_state;
        let global_state = &mut ctx.accounts.global_state;

        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time >= global_state.reveal_start && current_time <= global_state.reveal_end,
            ErrorCode::NotInRevealPeriod
        );

        let nft = user_state
            .nfts
            .iter_mut()
            .find(|n| n.mint == mint)
            .ok_or(ErrorCode::NftNotFound)?;

        if nft.revealed_number.is_some() {
            return Err(error!(ErrorCode::NftAlreadyRevealed));
        }

        let mut available_numbers: Vec<u8> = (1..=100)
            .filter(|n| !global_state.used_numbers.contains(n))
            .collect();
        if available_numbers.is_empty() {
            return Err(error!(ErrorCode::NoAvailableNumbers));
        }

        // Simulating randomness with deterministic behavior for now
        let random_number = available_numbers[0];
        nft.revealed_number = Some(random_number);
        global_state.used_numbers.push(random_number);

        Ok(())
    }
}

#[account]
pub struct GlobalState {
    pub total_nfts_minted: u64,
    pub max_nfts: u64,
    pub nft_price_lamports: u64,
    pub purchase_start: i64,
    pub purchase_end: i64,
    pub reveal_start: i64,
    pub reveal_end: i64,
    pub used_numbers: Vec<u8>,
    pub admin: Pubkey,
}

#[account]
pub struct UserState {
    pub nfts: Vec<NftInfo>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct NftInfo {
    pub mint: Pubkey,
    pub revealed_number: Option<u8>,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = 8 + 32 + 8 + 8 + 32 + 8 + 8 + 8 + 8 + 1 + 204)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetPeriods<'info> {
    #[account(mut, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct Purchase<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    #[account(init, payer = payer, space = 8 + 8 + (32 + 1) * 100)]
    pub user_state: Account<'info, UserState>,
    #[account(init, payer = payer, mint::decimals = 0, mint::authority = payer)]
    pub mint: Account<'info, Mint>,
    #[account(address = anchor_spl::associated_token::ID)]
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(init, payer = payer, associated_token::mint = mint, associated_token::authority = payer)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer_token_account: Account<'info, TokenAccount>, // Payer's token account
    #[account(mut)]
    pub admin_token_account: Account<'info, TokenAccount>, // Admin's token account
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


#[derive(Accounts)]
pub struct Reveal<'info> {
    #[account(mut)]
    pub user_state: Account<'info, UserState>,
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    pub payer: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("NFT limit reached.")]
    NftLimitReached,
    #[msg("Purchase period has not started or has ended.")]
    NotInPurchasePeriod,
    #[msg("Reveal period has not started or has ended.")]
    NotInRevealPeriod,
    #[msg("Invalid time periods provided.")]
    InvalidTimePeriods,
    #[msg("NFT not found.")]
    NftNotFound,
    #[msg("NFT has already been revealed.")]
    NftAlreadyRevealed, 
    #[msg("No available numbers to assign.")]
    NoAvailableNumbers,
}
