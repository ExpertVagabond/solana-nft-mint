# solana-nft-mint

NFT minting program with collection management and on-chain metadata on Solana.

![Rust](https://img.shields.io/badge/Rust-000000?logo=rust) ![Solana](https://img.shields.io/badge/Solana-9945FF?logo=solana&logoColor=white) ![Anchor](https://img.shields.io/badge/Anchor-blue) ![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)

## Overview

A Solana Anchor program for creating NFT collections and minting individual NFTs with on-chain metadata. An authority creates a collection with a name, symbol, and max supply. NFTs are minted as zero-decimal, supply-one SPL tokens with metadata stored in PDA accounts containing the creator, token ID, URI hash, and timestamp. The collection authority controls minting and can update metadata URI hashes after mint.

## Program Instructions

| Instruction | Description | Key Accounts |
|---|---|---|
| `create_collection` | Create a new NFT collection with name, symbol, and max supply | `authority` (signer), `collection` (PDA) |
| `mint_nft` | Mint a new NFT into the collection with a URI hash | `payer` (signer), `collection`, `nft_mint` (0-decimal, 0-supply), `nft_token_account`, `metadata` (PDA) |
| `update_metadata` | Update the URI hash of an existing NFT's metadata | `authority` (signer), `collection`, `metadata` |

## Account Structures

### Collection

| Field | Type | Description |
|---|---|---|
| `authority` | `Pubkey` | Collection admin who controls minting |
| `name` | `String` | Collection name (max 32 chars) |
| `symbol` | `String` | Collection symbol (max 10 chars) |
| `max_supply` | `u64` | Maximum number of NFTs in the collection |
| `current_supply` | `u64` | Number of NFTs minted so far |
| `bump` | `u8` | PDA bump seed |

### NftMetadata

| Field | Type | Description |
|---|---|---|
| `collection` | `Pubkey` | Parent collection |
| `mint` | `Pubkey` | NFT's SPL token mint address |
| `token_id` | `u64` | Sequential token ID within the collection |
| `creator` | `Pubkey` | Wallet that initiated the mint |
| `uri_hash` | `[u8; 32]` | SHA-256 hash of the off-chain metadata URI |
| `created_at` | `i64` | Unix timestamp of mint |
| `bump` | `u8` | PDA bump seed |

## PDA Seeds

- **Collection:** `["collection", authority]`
- **Metadata:** `["metadata", collection, nft_mint]`

## Error Codes

| Error | Description |
|---|---|
| `NameTooLong` | Collection name exceeds 32 characters |
| `SymbolTooLong` | Symbol exceeds 10 characters |
| `MaxSupplyReached` | Collection has reached its max supply |
| `Overflow` | Arithmetic overflow |

## Build & Test

```bash
anchor build
anchor test
```

## Deploy

```bash
solana config set --url devnet
anchor deploy
```

## License

[MIT](LICENSE)
