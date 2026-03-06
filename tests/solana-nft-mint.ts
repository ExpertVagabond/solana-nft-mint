import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaNftMint } from "../target/types/solana_nft_mint";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  getAccount,
} from "@solana/spl-token";
import { expect } from "chai";

describe("solana-nft-mint", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.solanaNftMint as Program<SolanaNftMint>;
  const authority = Keypair.generate();

  // Helpers
  const findCollectionPDA = (authorityKey: PublicKey): [PublicKey, number] => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("collection"), authorityKey.toBuffer()],
      program.programId
    );
  };

  const findMetadataPDA = (
    collectionKey: PublicKey,
    mintKey: PublicKey
  ): [PublicKey, number] => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("metadata"), collectionKey.toBuffer(), mintKey.toBuffer()],
      program.programId
    );
  };

  /** Create a zero-decimal mint owned by `collectionPDA` and a token account for `owner`. */
  const createNftMintAndAta = async (
    payer: Keypair,
    mintAuthority: PublicKey,
    owner: PublicKey
  ): Promise<{ nftMint: PublicKey; nftTokenAccount: PublicKey }> => {
    const nftMint = await createMint(
      provider.connection,
      payer,
      mintAuthority, // mint authority = collection PDA
      null, // no freeze authority
      0 // 0 decimals for NFT
    );
    const nftTokenAccount = await createAccount(
      provider.connection,
      payer,
      nftMint,
      owner
    );
    return { nftMint, nftTokenAccount };
  };

  before(async () => {
    const sig = await provider.connection.requestAirdrop(
      authority.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);
  });

  // ---------- create_collection ----------

  describe("create_collection", () => {
    it("initializes a collection with name, symbol, and max supply", async () => {
      const [collectionPDA] = findCollectionPDA(authority.publicKey);
      const name = "Test Collection";
      const symbol = "TNFT";
      const maxSupply = new anchor.BN(100);

      await program.methods
        .createCollection(name, symbol, maxSupply)
        .accounts({
          authority: authority.publicKey,
          collection: collectionPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      const collection = await program.account.collection.fetch(collectionPDA);
      expect(collection.authority.toBase58()).to.equal(
        authority.publicKey.toBase58()
      );
      expect(collection.name).to.equal(name);
      expect(collection.symbol).to.equal(symbol);
      expect(collection.maxSupply.toNumber()).to.equal(100);
      expect(collection.currentSupply.toNumber()).to.equal(0);
    });

    it("rejects a name longer than 32 characters", async () => {
      const other = Keypair.generate();
      const airdropSig = await provider.connection.requestAirdrop(
        other.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [collectionPDA] = findCollectionPDA(other.publicKey);
      const longName = "A".repeat(33);

      try {
        await program.methods
          .createCollection(longName, "X", new anchor.BN(1))
          .accounts({
            authority: other.publicKey,
            collection: collectionPDA,
            systemProgram: SystemProgram.programId,
          })
          .signers([other])
          .rpc();
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("NameTooLong");
      }
    });
  });

  // ---------- mint_nft ----------

  describe("mint_nft", () => {
    it("mints an NFT and increments current supply", async () => {
      const [collectionPDA] = findCollectionPDA(authority.publicKey);

      // Create mint with collection PDA as mint authority
      const { nftMint, nftTokenAccount } = await createNftMintAndAta(
        authority,
        collectionPDA, // collection PDA is the mint authority
        authority.publicKey
      );

      const [metadataPDA] = findMetadataPDA(collectionPDA, nftMint);
      const uriHash = Buffer.alloc(32, 0xab);

      await program.methods
        .mintNft([...uriHash] as any)
        .accounts({
          payer: authority.publicKey,
          collection: collectionPDA,
          nftMint: nftMint,
          nftTokenAccount: nftTokenAccount,
          metadata: metadataPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      // Verify supply incremented
      const collection = await program.account.collection.fetch(collectionPDA);
      expect(collection.currentSupply.toNumber()).to.equal(1);

      // Verify token account has 1 token
      const tokenAcct = await getAccount(provider.connection, nftTokenAccount);
      expect(Number(tokenAcct.amount)).to.equal(1);

      // Verify metadata
      const metadata = await program.account.nftMetadata.fetch(metadataPDA);
      expect(metadata.collection.toBase58()).to.equal(
        collectionPDA.toBase58()
      );
      expect(metadata.mint.toBase58()).to.equal(nftMint.toBase58());
      expect(metadata.tokenId.toNumber()).to.equal(0);
      expect(metadata.creator.toBase58()).to.equal(
        authority.publicKey.toBase58()
      );
    });

    it("mints a second NFT and supply becomes 2", async () => {
      const [collectionPDA] = findCollectionPDA(authority.publicKey);

      const { nftMint, nftTokenAccount } = await createNftMintAndAta(
        authority,
        collectionPDA,
        authority.publicKey
      );

      const [metadataPDA] = findMetadataPDA(collectionPDA, nftMint);
      const uriHash = Buffer.alloc(32, 0xcd);

      await program.methods
        .mintNft([...uriHash] as any)
        .accounts({
          payer: authority.publicKey,
          collection: collectionPDA,
          nftMint: nftMint,
          nftTokenAccount: nftTokenAccount,
          metadata: metadataPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      const collection = await program.account.collection.fetch(collectionPDA);
      expect(collection.currentSupply.toNumber()).to.equal(2);

      const metadata = await program.account.nftMetadata.fetch(metadataPDA);
      expect(metadata.tokenId.toNumber()).to.equal(1);
    });
  });

  // ---------- error: mint beyond max supply ----------

  describe("mint beyond max supply", () => {
    const limitedAuthority = Keypair.generate();

    before(async () => {
      const sig = await provider.connection.requestAirdrop(
        limitedAuthority.publicKey,
        10 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);

      // Create collection with max supply = 1
      const [collectionPDA] = findCollectionPDA(limitedAuthority.publicKey);
      await program.methods
        .createCollection("Limited", "LTD", new anchor.BN(1))
        .accounts({
          authority: limitedAuthority.publicKey,
          collection: collectionPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([limitedAuthority])
        .rpc();

      // Mint the one allowed NFT
      const { nftMint, nftTokenAccount } = await createNftMintAndAta(
        limitedAuthority,
        collectionPDA,
        limitedAuthority.publicKey
      );
      const [metadataPDA] = findMetadataPDA(collectionPDA, nftMint);
      const uriHash = Buffer.alloc(32, 0x01);

      await program.methods
        .mintNft([...uriHash] as any)
        .accounts({
          payer: limitedAuthority.publicKey,
          collection: collectionPDA,
          nftMint: nftMint,
          nftTokenAccount: nftTokenAccount,
          metadata: metadataPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([limitedAuthority])
        .rpc();
    });

    it("fails when minting beyond max supply", async () => {
      const [collectionPDA] = findCollectionPDA(limitedAuthority.publicKey);

      const { nftMint, nftTokenAccount } = await createNftMintAndAta(
        limitedAuthority,
        collectionPDA,
        limitedAuthority.publicKey
      );
      const [metadataPDA] = findMetadataPDA(collectionPDA, nftMint);
      const uriHash = Buffer.alloc(32, 0x02);

      try {
        await program.methods
          .mintNft([...uriHash] as any)
          .accounts({
            payer: limitedAuthority.publicKey,
            collection: collectionPDA,
            nftMint: nftMint,
            nftTokenAccount: nftTokenAccount,
            metadata: metadataPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([limitedAuthority])
          .rpc();
        expect.fail("should have thrown MaxSupplyReached");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("MaxSupplyReached");
      }
    });
  });
});
