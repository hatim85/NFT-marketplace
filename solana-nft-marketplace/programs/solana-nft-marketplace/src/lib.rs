use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer, Token};
use mpl_token_metadata::ID as METADATA_PROGRAM_ID;
use mpl_token_metadata::pda::find_metadata_account;
use mpl_token_metadata::types::{DataV2, Creator};
use mpl_token_metadata::instruction::create_metadata_accounts_v3;
use solana_program::program::invoke;

declare_id!("BLLM8SgKAAMRff8EDHgorDxgT1BFm9c6MeRZK1qP9BDg");

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
        // Find metadata PDA
        let (metadata_pda, _bump) = find_metadata_account(&ctx.accounts.mint.key());

        // Define creator
        let creators = vec![Creator {
            address: ctx.accounts.creator.key(),
            verified: true,  // Creator is verified
            share: 100,      // 100% ownership
        }];

        // Define NFT metadata
        let metadata_data = DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points,
            creators: Some(creators),
            collection: None,
            uses: None,
        };

        // Create metadata accounts v3 instruction
        let ix = create_metadata_accounts_v3(
            METADATA_PROGRAM_ID,
            metadata_pda,
            ctx.accounts.mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.payer.key(),
            ctx.accounts.creator.key(),
            metadata_data,
            true,  // is_mutable
            false, // update_authority_is_signer
            None,  // collection
            None,  // uses
        );

        // Invoke the instruction to create metadata
        invoke(
            &ix,
            &[
                ctx.accounts.token_metadata.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
        )?;

        // Initialize the mint
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_accounts = token::InitializeMint {
            mint: ctx.accounts.mint.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::initialize_mint(cpi_ctx, 0, &ctx.accounts.mint_authority.key(), None)?;

        // Mint the token to the payer's associated token account
        let cpi_accounts = token::MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.payer_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::mint_to(cpi_ctx, 1)?;

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

        // Transfer SOL from buyer to seller
        **ctx.accounts.seller.lamports.borrow_mut() += nft_account.price;
        **ctx.accounts.buyer.lamports.borrow_mut() -= nft_account.price;

        // Calculate and transfer royalties
        let royalty_amount = (nft_account.price * 5) / 100;
        **ctx.accounts.creator.lamports.borrow_mut() += royalty_amount;
        **ctx.accounts.seller.lamports.borrow_mut() -= royalty_amount;

        // Transfer NFT to buyer
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        // Update ownership
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
    pub payer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
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
    pub creator: Signer<'info>,
    #[account(mut)]
    pub nft_account: Account<'info, NFTAccount>,
    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
