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
use std::convert::TryInto;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

pub mod constant;
pub mod error;
use crate::error::ErrorCode;
use crate::constant::*;

declare_id!("GupsVjvKT1pv3KQ2cfgv2P2YB9SHeLLhT4HuvZC46Ajo");

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
        global_state.total_revealed = 0;
        global_state.max_nfts = max_nfts;
        global_state.purchase_start = purchase_start;
        global_state.purchase_end = purchase_end;
        global_state.reveal_start = reveal_start;
        global_state.reveal_end = reveal_end;
        global_state.admin = ctx.accounts.admin.key();
        global_state.admin_sol_account = ctx.accounts.admin_sol_account.key();
        global_state.treasury_account = ctx.accounts.treasury_account.key();

        Ok(())
    }

    pub fn initialize_random_state(ctx: Context<InitializeUsedNumber>, index: u16) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.used_numbers = Vec::new();
        state.index = index;
        state.start = index;
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

        let price_update = &mut ctx.accounts.price_update;
        // get_price_no_older_than will fail if the price update is more than 60 seconds old
        let maximum_age: u64 = 60;
        // This string is the id of the SOL/USD feed. See https://pyth.network/developers/price-feed-ids for all available IDs.
        let feed_id: [u8; 32] = get_feed_id_from_hex("0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d")?;
        let price = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        // Sample output:
        msg!("SOL/USD price is ({} ± {}) * 10^{}", price.price, price.conf, price.exponent);
     
        // Safely convert price.price (i64) to u64
        let price_value: u64 = price.price.try_into().expect("Negative price value");
        msg!("SOL/USD is {}", price_value);
        let NFT_PRICE = (PRICE / price_value) * 10_u64.pow(4) ;
        msg!("NFT price is {}", NFT_PRICE);
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
        let timestamp = Clock::get()?.unix_timestamp;
        let hash = anchor_lang::solana_program::keccak::hash(format!("{}", timestamp).as_bytes());
        let random_number = u16::from_le_bytes(hash.as_ref()[..2].try_into().unwrap()) % 1111 as u16 + ctx.accounts.state.start as u16;

        if random_number != 0 && !ctx.accounts.state.used_numbers.contains(&random_number) {
            ctx.accounts.state.used_numbers.push(random_number);
            user_nfts.revealed_number = random_number;
            msg!("NFT mint is {}", mint);
            msg!("Reveal number is {}", random_number);
        } else {
            let end = 1111 + ctx.accounts.state.start - 1;
            for i in ctx.accounts.state.index..=end {
                if !ctx.accounts.state.used_numbers.contains(&(i as u16)) {
                    ctx.accounts.state.used_numbers.push(i as u16);
                    ctx.accounts.state.index = i + 1;
                    user_nfts.revealed_number = i;
                    msg!("NFT mint is {}", mint);
                    msg!("Reveal number is {}", i);
                    break;
                }
            }
        }
        
        global_state.total_revealed += 1;

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
    pub total_revealed: i64,
    pub purchase_start: i64,
    pub purchase_end: i64,
    pub reveal_start: i64,
    pub reveal_end: i64,
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

#[account]
pub struct UsedRandomNumber {
    pub used_numbers: Vec<u16>,
    pub index: u16,
    pub start: u16
}

#[derive(Accounts)]
#[instruction(index: u16)]
pub struct InitializeUsedNumber<'info> {
    #[account(
        init,
        seeds = [b"state", index.to_le_bytes().as_ref()],
        bump,
        payer = signer,
        space = 8 + 4 + (2 * 1111)
    )]
    pub state: Account<'info, UsedRandomNumber>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
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
    pub price_update: Account<'info, PriceUpdateV2>,
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
    #[account(mut)]
    pub state: Account<'info, UsedRandomNumber>,
    pub payer: Signer<'info>,  
    pub system_program: Program<'info, System>
}