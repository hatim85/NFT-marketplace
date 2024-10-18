use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer, Token};
use mpl_token_metadata::ID as METADATA_PROGRAM_ID;
use mpl_token_metadata::instructions::{create_metadata_account_v3, create_master_edition_v3};
use mpl_token_metadata::types::{Creator, DataV2};
use anchor_lang::solana_program::program::invoke;

declare_id!("3HdoDQLWBSaH9JGK3DRy68gYvySjWFWyyxumTHQhSNqD");

#[program]
pub mod solana_nft_marketplace {
    use super::*;

    pub fn mint_nft(
        ctx: Context<MintNFT>,
        uri: String,
        name: String,
        symbol: String,
        seller_fee_basis_points: u16,
    ) -> Result<()> {
        // Find Metadata PDA
        let (metadata_pda, _bump) = Pubkey::find_program_address(
            &[b"metadata", METADATA_PROGRAM_ID.as_ref(), ctx.accounts.mint.key().as_ref()],
            &METADATA_PROGRAM_ID,
        );

        // Define creators
        let creators = vec![Creator {
            address: ctx.accounts.creator.key(),
            verified: true,
            share: 100,
        }];

        // Prepare metadata data
        let metadata_data = DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points,
            creators: Some(creators),
            collection: None,
            uses: None,
        };

        // Create Metadata Account Instruction
        let metadata_ix = create_metadata_account_v3(
            METADATA_PROGRAM_ID,
            metadata_pda,
            ctx.accounts.mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.payer.key(),
            ctx.accounts.payer.key(),
            name,
            symbol,
            uri,
            Some(creators),
            seller_fee_basis_points,
            true,  // update_authority_is_signer
            false, // is_mutable
            None,  // collection
            None,  // uses
        );

        invoke(
            &metadata_ix,
            &[
                ctx.accounts.token_metadata.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Create Master Edition
        let (master_edition_pda, _bump) = Pubkey::find_program_address(
            &[b"metadata", METADATA_PROGRAM_ID.as_ref(), ctx.accounts.mint.key().as_ref(), b"edition"],
            &METADATA_PROGRAM_ID,
        );

        let master_edition_ix = create_master_edition_v3(
            METADATA_PROGRAM_ID,
            master_edition_pda,
            ctx.accounts.mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.payer.key(),
            Some(0), // max_supply for non-fungible tokens
        );

        invoke(
            &master_edition_ix,
            &[
                ctx.accounts.token_metadata.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }

    pub fn list_nft(ctx: Context<ListNFT>, price: u64) -> Result<()> {
        let nft_account = &mut ctx.accounts.nft_account;
        nft_account.owner = ctx.accounts.seller.key();
        nft_account.price = price;
        nft_account.mint = ctx.accounts.mint.key();
        Ok(())
    }

    pub fn buy_nft(ctx: Context<BuyNFT>) -> Result<()> {
        let nft_account = &mut ctx.accounts.nft_account;
        require!(nft_account.price > 0, MarketplaceError::InvalidPrice);

        **ctx.accounts.seller.lamports.borrow_mut() += nft_account.price;
        **ctx.accounts.buyer.lamports.borrow_mut() -= nft_account.price;

        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        nft_account.owner = ctx.accounts.buyer.key();
        Ok(())
    }
}

#[account]
pub struct NFTAccount {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub price: u64,
}

#[error_code]
pub enum MarketplaceError {
    #[msg("Invalid price.")]
    InvalidPrice,
}

#[derive(Accounts)]
pub struct MintNFT<'info> {
    #[account(init, payer = payer, mint::decimals = 0, mint::authority = mint_authority)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub mint_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + 32 + 8 + 32)]
    pub token_metadata: UncheckedAccount<'info>,
    pub creator: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ListNFT<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, constraint = nft_account.owner == seller.key())]
    pub nft_account: Account<'info, NFTAccount>,
    pub mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct BuyNFT<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub nft_account: Account<'info, NFTAccount>,
    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
