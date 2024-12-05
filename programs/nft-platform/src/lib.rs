use anchor_lang::prelude::*;
use anchor_spl::metadata::{self, Metadata, CreateMetadataAccountsV3};
use mpl_token_metadata::types::DataV2;

use anchor_spl::{
    token::{self, Token, MintTo},
};

use solana_program::{
    program::invoke,
    system_instruction,
    account_info::AccountInfo,
};

pub mod constant;
pub mod error;
use crate::error::ErrorCode;
use crate::constant::*;

declare_id!("EagfLcPwqgVjgHu6ePBXkrh3Dejea6hsZxJ4wb4cDffR");

#[program]
pub mod nft_platform {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        max_nfts: u64,
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
        global_state.total_raised = 0;
        global_state.max_nfts = max_nfts;
        global_state.purchase_start = purchase_start;
        global_state.purchase_end = purchase_end;
        global_state.reveal_start = reveal_start;
        global_state.reveal_end = reveal_end;
        global_state.used_numbers = Vec::new();
        global_state.admin = ctx.accounts.admin.key();
        global_state.admin_sol_account = ctx.accounts.admin_sol_account.key();
        global_state.treasury_account = ctx.accounts.treasury_account.key();

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
        require!(
            current_time >= global_state.purchase_start && current_time <= global_state.purchase_end,
            ErrorCode::NotInPurchasePeriod
        );

        require!(
            ctx.accounts.admin_sol_account.key() == global_state.admin_sol_account,
            ErrorCode::InvalidAdminSolAccount
        );
        require!(
            ctx.accounts.treasury_account.key() == global_state.treasury_account,
            ErrorCode::InvalidTreasuryAccount
        );
    
        require!(
            global_state.total_nfts_minted < global_state.max_nfts,
            ErrorCode::NftLimitReached
        );
    
        // Ensure the payer has enough SOL
        let payer_balance = ctx.accounts.payer.to_account_info().lamports();
        require!(payer_balance >= NFT_PRICE, ErrorCode::InsufficientFunds);
    
        // Calculate the split amounts
        let amount_to_address_one = NFT_PRICE * 75 / 100;
        let amount_to_address_two = NFT_PRICE - amount_to_address_one;
    
        transfer_lamports(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.admin_sol_account.to_account_info(),
            amount_to_address_one,
        )?;

        transfer_lamports(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.treasury_account.to_account_info(),
            amount_to_address_two,
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
    
        // Create Metadata for the NFT
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
                symbol: "ASHU".to_string(),
                uri: "https://violet-rear-raven-459.mypinata.cloud/ipfs/QmNeT3HfrYyc1q3yzJG9RFwXKkaZKj1c86RhW8WVq7VDbu".to_string(),
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false,
            true,
            None,
        )?;
    
        user_nfts.owner = *ctx.accounts.payer.key;
        user_nfts.mint_key = ctx.accounts.mint_account.key();
        user_nfts.revealed_number = 0;
    
        global_state.total_nfts_minted += 1;
        global_state.total_raised += NFT_PRICE;
    
        Ok(())
    }
       

    pub fn reveal(ctx: Context<Reveal>, mint: Pubkey) -> Result<()> {
        let user_nfts = &mut ctx.accounts.user_nfts;
        let global_state = &mut ctx.accounts.global_state;

        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time >= global_state.reveal_start && current_time <= global_state.reveal_end,
            ErrorCode::NotInRevealPeriod
        );

        if user_nfts.mint_key != mint {
            return Err(ErrorCode::NftNotFound.into());
        }
        if user_nfts.revealed_number != 0 {
            return Err(ErrorCode::NftAlreadyRevealed.into());
        }

        let available_numbers: Vec<u16> = (1..=8888)
            .filter(|n| !global_state.used_numbers.contains(n))
            .collect();
        if available_numbers.is_empty() {
            return Err(error!(ErrorCode::NoAvailableNumbers));
        }

        // Simulating randomness with deterministic behavior for now
        let random_number = available_numbers[0];
        user_nfts.revealed_number = random_number;
        global_state.used_numbers.push(random_number);

        Ok(())
    }

}

fn transfer_lamports<'a>(from: AccountInfo<'a>, to: AccountInfo<'a>, amount: u64) -> Result<()> {
    let ix = system_instruction::transfer(&from.key(), &to.key(), amount);
    invoke(&ix, &[from, to])?;

    Ok(())
}


#[account]
#[derive(InitSpace)]
pub struct GlobalState {
    pub total_nfts_minted: u64,
    pub max_nfts: u64,
    pub total_raised: u64,
    pub purchase_start: i64,
    pub purchase_end: i64,
    pub reveal_start: i64,
    pub reveal_end: i64,
    #[max_len(200)]
    pub used_numbers: Vec<u16>,
    pub admin: Pubkey,
    pub admin_sol_account: Pubkey,
    pub treasury_account: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct UserNFTs  {
    pub owner: Pubkey,           
    pub mint_key: Pubkey,              
    pub revealed_number: u16,   
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
    pub admin_sol_account: UncheckedAccount<'info>,      
    pub treasury_account: UncheckedAccount<'info>,
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
    pub token_metadata_program: Program<'info, Metadata>,
    #[account(mut)]
    pub admin_sol_account: UncheckedAccount<'info>,  
    #[account(mut)]    
    pub treasury_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Reveal<'info> {
    #[account(mut)]
    pub user_nfts: Account<'info, UserNFTs>,  
    #[account(mut)]
    pub global_state: Account<'info, GlobalState>,
    pub payer: Signer<'info>,  
    pub system_program: Program<'info, System>
}