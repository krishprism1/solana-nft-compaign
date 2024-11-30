import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftPlatform } from "../target/types/nft_platform";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAssociatedTokenAddress, createInitializeMintInstruction, createAssociatedTokenAccountInstruction, MINT_SIZE } from "@solana/spl-token";

describe("nft-platform", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;

  const program = anchor.workspace.NftPlatform as Program<NftPlatform>;

  let globalState: PublicKey;
  //@ts-ignore
  let admin = provider.wallet as Wallet;;
  let user = Keypair.generate();
  let mint: PublicKey;
  let userTokenAccount: PublicKey;
  let adminTokenAccount: PublicKey;

  const MAX_NFTS = 100;
  const NFT_PRICE_LAMPORTS = 1_000_000; // Price in lamports
  const PURCHASE_START = Math.floor(Date.now() / 1000); // Current time in seconds
  const PURCHASE_END = PURCHASE_START + 3600; // 1 hour later
  const REVEAL_START = PURCHASE_END + 3600; // 1 hour after purchase ends
  const REVEAL_END = REVEAL_START + 3600; // 1 hour later

  it("Initializes the program", async () => {
    const [globalStatePDA] = await PublicKey.findProgramAddress(
      [Buffer.from("NFT_PLATFORM")],
      program.programId
    );
    // await program.methods
    //   .initialize(
    //     new anchor.BN(MAX_NFTS),
    //     new anchor.BN(NFT_PRICE_LAMPORTS),
    //     new anchor.BN(PURCHASE_START),
    //     new anchor.BN(PURCHASE_END),
    //     new anchor.BN(REVEAL_START),
    //     new anchor.BN(REVEAL_END)
    //   )
    //   .accounts({
    //     globalState: globalStatePDA,
    //     admin: admin.PublicKey,
    //     systemProgram: SystemProgram.programId,
    //   })
    //   .signers([])
    //   .rpc();

    const state = await program.account.globalState.all();
    console.log("Global State:", state);
  });

  it("Purchases an NFT", async () => {
    const [globalStatePDA] = await PublicKey.findProgramAddress(
      [Buffer.from("NFT_PLATFORM")],
      program.programId
    );

    const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

    const mintKeypair: anchor.web3.Keypair = anchor.web3.Keypair.generate();
    const mintAccount = mintKeypair.publicKey;
    const associateTokenAccount = await anchor.utils.token.associatedAddress({
      mint: mintAccount,
      owner: admin.publicKey
    })

    const lamports: number = await program.provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE);
    const mint_tx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: mintAccount,
        space: MINT_SIZE,
        programId: TOKEN_PROGRAM_ID,
        lamports,
      }),
      createInitializeMintInstruction(mintAccount, 0, admin.publicKey, admin.publicKey),
      createAssociatedTokenAccountInstruction(admin.publicKey, associateTokenAccount, admin.publicKey, mintAccount),
    );

    const res = await program.provider.sendAndConfirm(mint_tx, [mintKeypair]);
    const vaultAccount = new PublicKey("AmQ1f82eQVJeAy8fQ8UzcYQQ79nTvTy5FssvMfSmBzP")
    const tokenAccount = new PublicKey("CWjKYqg7yucQURrCsMrdK3MpJm2H3YZAiBqY45BkB8vD");
    

    
    const getMetadata = async (data) => {
      const seed = await PublicKey.findProgramAddress(
        data,
        TOKEN_METADATA_PROGRAM_ID
      );
      return seed[0]
    }

    const metadataAddress = await getMetadata([Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBuffer(), mintAccount.toBuffer()])

    
    await program.methods
      .purchase()
      .accounts({
        globalState : globalStatePDA,
        payer: admin.publicKey,
        mintAccount:mintAccount,
        associatedTokenAccount: associateTokenAccount,
        payerTokenAccount: tokenAccount,
        adminTokenAccount : vaultAccount,
        // metadataAccount: metadataAddress,
        tokenProgram: TOKEN_PROGRAM_ID,
        // tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY
      })
      .signers([])
      .rpc();

    const userState = await program.account.userState.all();
    console.log("User State:", userState);
    const state = await program.account.globalState.all();
    console.log("Global State:", state);
  });
});
