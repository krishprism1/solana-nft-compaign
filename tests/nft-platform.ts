import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftPlatform } from "../target/types/nft_platform";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAssociatedTokenAddress, createInitializeMintInstruction, createAssociatedTokenAccountInstruction, MINT_SIZE } from "@solana/spl-token";
// import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";

describe("nft-platform", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;

  const program = anchor.workspace.NftPlatform as Program<NftPlatform>;
  //@ts-ignore
  let admin = provider.wallet as Wallet;
  //@ts-ignore
  // const pythSolanaReceiver = new PythSolanaReceiver({ connection, admin });

  const adminSolAccount = new PublicKey("HrFjzjeZHiQ3Goro6UxQL2gPiQrPAhmdkT7kUKZdZ3Fm");
  const treasuryAccount = new PublicKey("ECBME3yBHmff6sKdcUJadjXn8UPvVT65Srzmkj1Wu45r");
  const solUsdPriceFeedAccount = new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE")


  const MAX_NFTS = 8888;
  const PURCHASE_START = 1733939106; // Current time in seconds
  const PURCHASE_END = PURCHASE_START + 15 * 3600; // 1 hour later
  const REVEAL_START = PURCHASE_END 
  const REVEAL_END = REVEAL_START + 5* 3600; // 1 hour later

  it("Initializes the program", async () => {
    const [globalStatePDA] = await PublicKey.findProgramAddress(
      [Buffer.from("NFT_PLATFORM")],
      program.programId
    );
    await program.methods
      .initialize(
        new anchor.BN(MAX_NFTS),
        new anchor.BN(PURCHASE_START),
        new anchor.BN(PURCHASE_END),
        new anchor.BN(REVEAL_START),
        new anchor.BN(REVEAL_END)
      )
      .accounts({
        globalState: globalStatePDA,
        admin: admin.PublicKey,
        adminSolAccount: adminSolAccount,
        treasuryAccount: treasuryAccount,
        systemProgram: SystemProgram.programId,
      })
      .signers([])
      .rpc();

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

    const [userNfts] = await PublicKey.findProgramAddress(
      [mintAccount.toBuffer(), admin.publicKey.toBuffer()],
      program.programId
    );

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
        globalState: globalStatePDA,
        userNfts,
        payer: admin.publicKey,
        mintAccount: mintAccount,
        associatedTokenAccount: associateTokenAccount,
        adminSolAccount,
        treasuryAccount,
        metadataAccount: metadataAddress,
        priceUpdate: solUsdPriceFeedAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY
      })
      .signers([])
      .rpc();

    const userState = await program.account.userNfTs.all();
    console.log("User State:", userState);
    const state = await program.account.globalState.all();
    console.log("Global State:", state);
  });

  // it("Reveals NFTs", async () => {
  //   const [globalStatePDA] = await PublicKey.findProgramAddress(
  //     [Buffer.from("NFT_PLATFORM")],
  //     program.programId
  //   );

  //   const mint = new PublicKey("FaRroQmDgmLZxRh8gLYCtsDRCBF4QsqzmvEr914h7Ey3")
  //   const [userNfts] = await PublicKey.findProgramAddress(
  //     [mint.toBuffer(), admin.publicKey.toBuffer()],
  //     program.programId
  //   );

  //   await program.methods
  //     .reveal(mint)
  //     .accounts({
  //       globalState: globalStatePDA,
  //       userNfts,
  //       payer: admin.publicKey,
  //       systemProgram: SystemProgram.programId,
  //     })
  //     .signers([])
  //     .rpc()

  //   const userState = await program.account.userNfTs.all();
  //   console.log("Updated User State after Reveal:", userState);

  //   const globalState = await program.account.globalState.all();
  //   console.log("Updated Global State after Reveal:", globalState);

  // });

});
