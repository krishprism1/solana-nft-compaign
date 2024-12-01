use anchor_lang::prelude::*;
use anchor_spl::metadata::{self, Metadata, MetadataAccount, CreateMetadataAccountsV3};
use mpl_token_metadata::types::DataV2;

use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Token, TokenAccount, Transfer, Mint, MintTo},
};


pub mod constant;
pub mod error;
use crate::error::ErrorCode;
use crate::constant::*;

declare_id!("7ond1Kt7DFPa45wxPwFiLC2qDXUryrMX6igfBofJXW6y");

#[program]
pub mod nft_platform {
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
        let user_nfts = &mut ctx.accounts.user_nfts;
    
        let current_time = Clock::get()?.unix_timestamp;
        // require!(
        //     current_time >= global_state.purchase_start && current_time <= global_state.purchase_end,
        //     ErrorCode::NotInPurchasePeriod
        // );
    
        require!(
            global_state.total_nfts_minted < global_state.max_nfts,
            ErrorCode::NftLimitReached
        );
    
        // Transfer SPL tokens from the payer to the admin
        let nft_price = 1000; //global_state.nft_price_lamports; // Update this variable name if using SPL token price
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
            mint: ctx.accounts.mint_account.to_account_info(),
            to: ctx.accounts.associated_token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::mint_to(
            CpiContext::new(cpi_program, cpi_accounts),
            1, // Mint 1 NFT
        )?;
        metadata::create_metadata_accounts_v3(
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    mint_authority: ctx.accounts.payer.to_account_info(),
                    update_authority: ctx.accounts.payer.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            DataV2 {
                name: "NFTOne".to_string(),
                symbol: "NO".to_string(),
                uri: "https://avatars.githubusercontent.com/u/65070737?v=4".to_string(),
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false,
            true,
            None,
        )?;
    
        // // Record NFT in user's state
        // user_state.nfts.push(NftInfo {
        //     mint: ctx.accounts.mint_account.key(),
        //     revealed_number: None,
        // });

        user_nfts.owner = *ctx.accounts.payer.key;        
        user_nfts.mint_key = ctx.accounts.mint_account.key();        
        user_nfts.revealed_number = 0;
        
        global_state.total_nfts_minted += 1;
    
        Ok(())
    }    

    // pub fn reveal(ctx: Context<Reveal>, mint: Pubkey) -> Result<()> {
    //     let user_state = &mut ctx.accounts.user_state;
    //     let global_state = &mut ctx.accounts.global_state;

    //     let current_time = Clock::get()?.unix_timestamp;
    //     require!(
    //         current_time >= global_state.reveal_start && current_time <= global_state.reveal_end,
    //         ErrorCode::NotInRevealPeriod
    //     );

    //     let nft = user_state
    //         .nfts
    //         .iter_mut()
    //         .find(|n| n.mint == mint)
    //         .ok_or(ErrorCode::NftNotFound)?;

    //     if nft.revealed_number.is_some() {
    //         return Err(error!(ErrorCode::NftAlreadyRevealed));
    //     }

    //     let available_numbers: Vec<u8> = (1..=100)
    //         .filter(|n| !global_state.used_numbers.contains(n))
    //         .collect();
    //     if available_numbers.is_empty() {
    //         return Err(error!(ErrorCode::NoAvailableNumbers));
    //     }

    //     // Simulating randomness with deterministic behavior for now
    //     let random_number = available_numbers[0];
    //     nft.revealed_number = Some(random_number);
    //     global_state.used_numbers.push(random_number);

    //     Ok(())
    // }

}


#[account]
#[derive(InitSpace)]
pub struct GlobalState {
    pub total_nfts_minted: u64,
    pub max_nfts: u64,
    pub nft_price_lamports: u64,
    pub purchase_start: i64,
    pub purchase_end: i64,
    pub reveal_start: i64,
    pub reveal_end: i64,
    #[max_len(200)]
    pub used_numbers: Vec<u8>,
    pub admin: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct UserNFTs  {
    pub owner: Pubkey,           
    pub mint_key: Pubkey,              
    pub revealed_number: u8,   
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        seeds=[PREFIX],
        bump,
        payer = admin,
        space = 8 + GlobalState::INIT_SPACE,
    )]
    pub global_state: Account<'info, GlobalState>,
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
    #[account(mut)]
    pub mint_account: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        seeds= [mint_account.key().as_ref(), payer.key().as_ref()],
        bump,
        space= 8 + UserNFTs::INIT_SPACE,
    )]
    pub user_nfts: Account<'info, UserNFTs>,
    #[account(mut)]
    pub associated_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer_token_account: Account<'info, TokenAccount>, 
    #[account(mut)]
    pub admin_token_account: Account<'info, TokenAccount>,
    pub token_metadata_program: Program<'info, Metadata>,
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// #[derive(Accounts)]
// pub struct Reveal<'info> {
//     #[account(mut)]
//     pub user_state: Account<'info, UserState>,
//     #[account(mut)]
//     pub global_state: Account<'info, GlobalState>,
//     pub payer: Signer<'info>,
// }