use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface, TokenAccount, MintTo, mint_to};
use solana_nft_gated::cpi::accounts::Access as GatedAccess;
use solana_nft_gated::program::SolanaNftGated;

declare_id!("FxgTrgwz1fZNi5ypcoEFw9YJKYSMj7EdZemfHJuNU2zL");

#[program]
pub mod solana_nft_mint {
    use super::*;

    pub fn create_collection(ctx: Context<CreateCollection>, name: String, symbol: String, max_supply: u64) -> Result<()> {
        require!(name.len() <= 32, NftError::NameTooLong);
        require!(symbol.len() <= 10, NftError::SymbolTooLong);
        let collection = &mut ctx.accounts.collection;
        collection.authority = ctx.accounts.authority.key();
        collection.name = name;
        collection.symbol = symbol;
        collection.max_supply = max_supply;
        collection.current_supply = 0;
        collection.bump = ctx.bumps.collection;

        emit!(CollectionCreated {
            collection: collection.key(),
            authority: collection.authority,
            name: collection.name.clone(),
            max_supply: collection.max_supply,
        });

        Ok(())
    }

    pub fn mint_nft(ctx: Context<MintNft>, uri_hash: [u8; 32]) -> Result<()> {
        let collection = &mut ctx.accounts.collection;
        require!(collection.current_supply < collection.max_supply, NftError::MaxSupplyReached);

        let token_id = collection.current_supply;
        collection.current_supply = token_id.checked_add(1).ok_or(NftError::Overflow)?;

        let authority_key = collection.authority;
        let collection_key = collection.key();
        let bump = collection.bump;
        let seeds: &[&[u8]] = &[b"collection", authority_key.as_ref(), &[bump]];

        // Mint one token to the receiver
        mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.nft_mint.to_account_info(),
                to: ctx.accounts.nft_token_account.to_account_info(),
                authority: ctx.accounts.collection.to_account_info(),
            },
            &[seeds],
        ), 1)?;

        let metadata = &mut ctx.accounts.metadata;
        metadata.collection = collection_key;
        metadata.mint = ctx.accounts.nft_mint.key();
        metadata.token_id = token_id;
        metadata.creator = ctx.accounts.payer.key();
        metadata.uri_hash = uri_hash;
        metadata.created_at = Clock::get()?.unix_timestamp;
        metadata.bump = ctx.bumps.metadata;

        emit!(NftMinted {
            collection: collection_key,
            nft: ctx.accounts.nft_mint.key(),
            authority: ctx.accounts.payer.key(),
            token_id,
        });

        Ok(())
    }

    /// Mint an NFT and register the holder for gated access via CPI.
    pub fn mint_and_register(ctx: Context<MintAndRegister>, uri_hash: [u8; 32]) -> Result<()> {
        let collection = &mut ctx.accounts.collection;
        require!(collection.current_supply < collection.max_supply, NftError::MaxSupplyReached);

        let token_id = collection.current_supply;
        collection.current_supply = token_id.checked_add(1).ok_or(NftError::Overflow)?;

        let authority_key = collection.authority;
        let collection_key = collection.key();
        let bump = collection.bump;
        let seeds: &[&[u8]] = &[b"collection", authority_key.as_ref(), &[bump]];

        mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.nft_mint.to_account_info(),
                to: ctx.accounts.nft_token_account.to_account_info(),
                authority: ctx.accounts.collection.to_account_info(),
            },
            &[seeds],
        ), 1)?;

        let metadata = &mut ctx.accounts.metadata;
        metadata.collection = collection_key;
        metadata.mint = ctx.accounts.nft_mint.key();
        metadata.token_id = token_id;
        metadata.creator = ctx.accounts.payer.key();
        metadata.uri_hash = uri_hash;
        metadata.created_at = Clock::get()?.unix_timestamp;
        metadata.bump = ctx.bumps.metadata;

        // CPI into nft-gated to register access
        let cpi_accounts = GatedAccess {
            holder: ctx.accounts.payer.to_account_info(),
            gate: ctx.accounts.gate.to_account_info(),
            holder_token_account: ctx.accounts.nft_token_account.to_account_info(),
            access_record: ctx.accounts.access_record.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        solana_nft_gated::cpi::access(
            CpiContext::new(ctx.accounts.nft_gated_program.to_account_info(), cpi_accounts),
        )?;

        emit!(NftMintedAndRegistered {
            collection: collection_key,
            nft: ctx.accounts.nft_mint.key(),
            holder: ctx.accounts.payer.key(),
            token_id,
            gate: ctx.accounts.gate.key(),
        });

        Ok(())
    }

    pub fn update_metadata(ctx: Context<UpdateMetadata>, new_uri_hash: [u8; 32]) -> Result<()> {
        ctx.accounts.metadata.uri_hash = new_uri_hash;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + Collection::INIT_SPACE,
        seeds = [b"collection", authority.key().as_ref()], bump)]
    pub collection: Account<'info, Collection>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"collection", collection.authority.as_ref()], bump = collection.bump)]
    pub collection: Account<'info, Collection>,
    #[account(mut, constraint = nft_mint.decimals == 0, constraint = nft_mint.supply == 0)]
    pub nft_mint: InterfaceAccount<'info, Mint>,
    #[account(mut, constraint = nft_token_account.mint == nft_mint.key())]
    pub nft_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(init, payer = payer, space = 8 + NftMetadata::INIT_SPACE,
        seeds = [b"metadata", collection.key().as_ref(), nft_mint.key().as_ref()], bump)]
    pub metadata: Account<'info, NftMetadata>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintAndRegister<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"collection", collection.authority.as_ref()], bump = collection.bump)]
    pub collection: Account<'info, Collection>,
    #[account(mut, constraint = nft_mint.decimals == 0, constraint = nft_mint.supply == 0)]
    pub nft_mint: InterfaceAccount<'info, Mint>,
    #[account(mut, constraint = nft_token_account.mint == nft_mint.key())]
    pub nft_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(init, payer = payer, space = 8 + NftMetadata::INIT_SPACE,
        seeds = [b"metadata", collection.key().as_ref(), nft_mint.key().as_ref()], bump)]
    pub metadata: Account<'info, NftMetadata>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    /// CHECK: Validated by the nft-gated program via CPI
    #[account(mut)]
    pub gate: UncheckedAccount<'info>,
    /// CHECK: Initialized by the nft-gated program via CPI
    #[account(mut)]
    pub access_record: UncheckedAccount<'info>,
    pub nft_gated_program: Program<'info, SolanaNftGated>,
}

#[derive(Accounts)]
pub struct UpdateMetadata<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [b"collection", collection.authority.as_ref()], bump = collection.bump, has_one = authority)]
    pub collection: Account<'info, Collection>,
    #[account(mut, has_one = collection)]
    pub metadata: Account<'info, NftMetadata>,
}

#[account]
#[derive(InitSpace)]
pub struct Collection {
    pub authority: Pubkey,
    #[max_len(32)]
    pub name: String,
    #[max_len(10)]
    pub symbol: String,
    pub max_supply: u64,
    pub current_supply: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct NftMetadata {
    pub collection: Pubkey,
    pub mint: Pubkey,
    pub token_id: u64,
    pub creator: Pubkey,
    pub uri_hash: [u8; 32],
    pub created_at: i64,
    pub bump: u8,
}

#[event]
pub struct CollectionCreated {
    pub collection: Pubkey,
    pub authority: Pubkey,
    pub name: String,
    pub max_supply: u64,
}

#[event]
pub struct NftMinted {
    pub collection: Pubkey,
    pub nft: Pubkey,
    pub authority: Pubkey,
    pub token_id: u64,
}

#[event]
pub struct NftMintedAndRegistered {
    pub collection: Pubkey,
    pub nft: Pubkey,
    pub holder: Pubkey,
    pub token_id: u64,
    pub gate: Pubkey,
}

#[error_code]
pub enum NftError {
    #[msg("Name too long (max 32)")]
    NameTooLong,
    #[msg("Symbol too long (max 10)")]
    SymbolTooLong,
    #[msg("Max supply reached")]
    MaxSupplyReached,
    #[msg("Overflow")]
    Overflow,
}
