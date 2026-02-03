import * as anchor from "@coral-xyz/anchor";
import * as path from "path";
import { Program } from "@coral-xyz/anchor";
import { Kickstarter } from "../target/types/kickstarter";
import {
  getAuthToken,
  MAGIC_CONTEXT_ID,
} from "@magicblock-labs/ephemeral-rollups-sdk";
import * as MagicBlockSdk from '@magicblock-labs/ephemeral-rollups-sdk';
import kickstarterIdl from "../frontend-kickstarter/idl/kickstarter.json";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  AuthorityType,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  setAuthority,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  SystemProgram,
  PublicKey,
  Keypair,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { sign } from "tweetnacl";
import * as fs from "fs";
import { BN } from '@coral-xyz/anchor';


// GLOBAL PROGRAMS
const DEFAULT_METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');
// SEEDS
const SEED_KICKSTARTER = 'kickstarter';
const SEED_BASE_VAULT = 'base_vault';
const SEED_QUOTE_VAULT = 'quote_vault';
const SEED_FUNDER_POSITION = 'funder_position';

// HELPERS
const addLog = (msg: string) => console.log(`[${new Date().toLocaleTimeString()}] ${msg}`);


describe("kickstarter", () => {
  const readKeypairFromCurrentDir = (fileName: string) => {
    const filePath = path.join(__dirname, fileName);
    const secret = JSON.parse(fs.readFileSync(filePath, "utf-8"));
    return Keypair.fromSecretKey(new Uint8Array(secret));
  };

  const adminKeypair = readKeypairFromCurrentDir("admin-keypair.json");
  const wallet = new anchor.Wallet(adminKeypair);
  const investorKeypair = readKeypairFromCurrentDir("investor-keypair.json");
  console.log({
    adminPublicKey: adminKeypair.publicKey.toBase58(),
    investorPublicKey: investorKeypair.publicKey.toBase58(),
  })

  const solanaProvider = new anchor.AnchorProvider(
    // Original
    // new anchor.web3.Connection("https://rpc.magicblock.app/devnet", {
    //   wsEndpoint: "wss://rpc.magicblock.app/devnet",
    // }),
    new anchor.web3.Connection("https://api.devnet.solana.com", {
      wsEndpoint: "wss://api.devnet.solana.com",
    }),
    wallet
  );
  anchor.setProvider(solanaProvider);

  const solanaProgram = new Program<Kickstarter>(kickstarterIdl, solanaProvider);
  let ephemeralProgramAdmin: Program<Kickstarter>;
  let ephemeralProgramInvestor: Program<Kickstarter>;

  const admin = adminKeypair.publicKey;
  const investor = investorKeypair.publicKey;

  let metadataPda: PublicKey;
  let treasury: PublicKey;
  // My program accounts
  let kickstarterPda: PublicKey;
  let privateState: PublicKey;

  let baseMint: PublicKey;
  let quoteMint: PublicKey;
  let baseVaultPda: PublicKey;
  let quoteVaultPda: PublicKey;

  const privateFundAmount = '100'
  const amount = parseFloat(privateFundAmount) * 1_000_000;

  let ephemeralRpcUrl = "https://tee.magicblock.app";

  before(async () => {

    const { token } = await getAuthToken(ephemeralRpcUrl, wallet.publicKey, async (message) => sign.detached(message, adminKeypair.secretKey));
    const ephemeralProviderAdmin = new anchor.AnchorProvider(
      new anchor.web3.Connection("https://tee.magicblock.app?token=" + token, {
        wsEndpoint: "wss://tee.magicblock.app?token=" + token,
      }),
      wallet
    );
    ephemeralProgramAdmin = new Program<Kickstarter>(
      kickstarterIdl,
      ephemeralProviderAdmin
    );

    const { token: otherToken } = await getAuthToken(ephemeralRpcUrl, investorKeypair.publicKey, async (message) => sign.detached(message, investorKeypair.secretKey));
    const ephemeralProviderInvestor = new anchor.AnchorProvider(
      new anchor.web3.Connection("https://tee.magicblock.app?token=" + otherToken, {
        wsEndpoint: "wss://tee.magicblock.app?token=" + otherToken,
      }),
      new anchor.Wallet(investorKeypair)
    );
    ephemeralProgramInvestor = new Program<Kickstarter>(
      kickstarterIdl,
      ephemeralProviderInvestor
    );

    let balance = await solanaProvider.connection.getBalance(adminKeypair.publicKey);
    console.log("Balance", balance);
    while (balance === 0) {
      console.log("Airdropping...");
      await new Promise((resolve) => setTimeout(resolve, 1000));
      balance = await solanaProvider.connection.getBalance(adminKeypair.publicKey);
    }
    if (balance === 0) throw new Error("airdrop failed...");

    addLog('Creating Base Mint...');
    baseMint = await createMint(solanaProvider.connection, adminKeypair, admin, admin, 6);
    addLog(`Base Mint: ${baseMint.toBase58()}`);

    addLog('Creating Quote Mint...');
    quoteMint = await createMint(solanaProvider.connection, adminKeypair, admin, null, 6);
    addLog(`Quote Mint: ${quoteMint.toBase58()}`);

    for (const { mint, name } of [{ mint: baseMint, name: 'base' }, { mint: quoteMint, name: 'quote' }]) {
      while ((await solanaProvider.connection.getAccountInfo(mint)) === null) {
        console.log(`Waiting for ${name} mint to be created...`);
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    }

    [kickstarterPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(SEED_KICKSTARTER), admin.toBuffer(), baseMint.toBuffer()],
      solanaProgram.programId
    );
    addLog(`Kickstarter PDA: ${kickstarterPda.toBase58()}`);

    [privateState] = PublicKey.findProgramAddressSync(
      [Buffer.from('private_state'), kickstarterPda.toBuffer()],
      solanaProgram.programId
    );
    addLog(`Private State PDA: ${privateState.toBase58()}`);

    [baseVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(SEED_BASE_VAULT), kickstarterPda.toBuffer()],
      solanaProgram.programId
    );
    [quoteVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(SEED_QUOTE_VAULT), kickstarterPda.toBuffer()],
      solanaProgram.programId
    );
    treasury = admin;

    [metadataPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('metadata'), DEFAULT_METADATA_PROGRAM_ID.toBuffer(), baseMint.toBuffer()],
      DEFAULT_METADATA_PROGRAM_ID
    );
    addLog('Transferring Mint Authority to PDA...');
    await setAuthority(solanaProvider.connection, adminKeypair, baseMint, adminKeypair.publicKey, AuthorityType.MintTokens, kickstarterPda);

    addLog('Transferring Freeze Authority to PDA...');
    await setAuthority(solanaProvider.connection, adminKeypair, baseMint, adminKeypair.publicKey, AuthorityType.FreezeAccount, kickstarterPda);



    console.log("Creating user token accounts...");
    const investorAta = await getOrCreateAssociatedTokenAccount(solanaProvider.connection, adminKeypair, quoteMint, investor);
    await mintTo(solanaProvider.connection, adminKeypair, quoteMint, investorAta.address, adminKeypair, amount * 2);

    while (
      (await solanaProvider.connection.getAccountInfo(investorAta.address)) === null
    ) {
      console.log(`Waiting for investor ata to be created...`);
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  });

  it("Initialize Kickstarter", async () => {
      const initParams = {
        tokenName: "Demo Token",
        tokenSymbol: "DEMO",
        tokenUri: "https://arweave.net/123",
        minRaise: "200",
        teamSpending: "1",
        launchDuration: "86400",
        totalTokens: "10",
        perfPool: "2",
        packageUnlockDelay: "60"
      }
      addLog('Sending initialize_kickstarter...');

      const tx = await solanaProgram.methods
        .initializeKickstarter(
          new BN(parseFloat(initParams.minRaise) * 1_000_000),
          new BN(parseFloat(initParams.totalTokens) * 1_000_000 * 1_000_000),
          new BN(parseFloat(initParams.perfPool) * 1_000_000 * 1_000_000),
          parseInt(initParams.launchDuration),
          new BN(parseFloat(initParams.teamSpending) * 1_000_000),
          new BN(parseInt(initParams.packageUnlockDelay)),
          initParams.tokenName,
          initParams.tokenSymbol,
          "Description",
          initParams.tokenUri
        )
        .accounts({
          admin: adminKeypair.publicKey,
          // @ts-ignore
          kickstarter: kickstarterPda,
          baseMint: baseMint,
          quoteMint: quoteMint,
          baseVault: baseVaultPda,
          quoteVault: quoteVaultPda,
          treasury: treasury,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
          metadata: metadataPda,
          tokenMetadataProgram: DEFAULT_METADATA_PROGRAM_ID,
        })
        .signers([adminKeypair])
        .rpc();

      addLog(`SUCCESS! Tx: ${tx}`);
  });

  it("Start Campaign", async () => {
    const tx = await solanaProgram.methods
      .startKickstarter()
      .accounts({ admin: adminKeypair.publicKey, kickstarter: kickstarterPda })
      .signers([adminKeypair])
      .rpc();

    addLog(`START SUCCESS! Tx: ${tx}`);
  })

  it("Start private round", async () => {
    const tx = await solanaProgram.methods
      .startPrivateRound()
      .accounts({ admin: adminKeypair.publicKey, kickstarter: kickstarterPda })
      .signers([adminKeypair])
      .rpc();
    
    addLog(`PRIVATE ROUND STARTED! Tx: ${tx}`);
  });

  it('Create permission for investor', async () => {
      const PERMISSION_FLAGS =
        MagicBlockSdk.AUTHORITY_FLAG |
        MagicBlockSdk.TX_LOGS_FLAG |
        MagicBlockSdk.TX_BALANCES_FLAG |
        MagicBlockSdk.TX_MESSAGE_FLAG |
        MagicBlockSdk.ACCOUNT_SIGNATURES_FLAG;
      const members = [
        { pubkey: investor, flags: PERMISSION_FLAGS },
        { pubkey: adminKeypair.publicKey, flags: PERMISSION_FLAGS },
      ];
      const privateStatePermission = MagicBlockSdk.permissionPdaFromAccount(privateState);

      const tx = await solanaProgram.methods
          .createPermission({ privateState: { kickstarter: kickstarterPda } }, members)
          .accounts({
            permissionedAccount: privateState,
            permission: privateStatePermission,
            payer: adminKeypair.publicKey,
            // @ts-ignore
            permissionProgram: MagicBlockSdk.PERMISSION_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([adminKeypair])
          .rpc();
    addLog(`Permission created! Tx: ${tx}`);
    const permissionStatus = await MagicBlockSdk.waitUntilPermissionActive(ephemeralProgramInvestor.provider.connection.rpcEndpoint, privateState);
    addLog(`is permission active: ${permissionStatus}`);
  })

  it("Delegate", async () => {
    const validator = new PublicKey("FnE6VJT5QNZdedZPnCoLsARgBwoE6DeJNjBs2H1gySXA");
    const delegateAccounts = {
      bufferPda: MagicBlockSdk.delegateBufferPdaFromDelegatedAccountAndOwnerProgram(
        privateState,
        solanaProgram.programId
      ),
      delegationRecordPda: MagicBlockSdk.delegationRecordPdaFromDelegatedAccount(privateState),
      delegationMetadataPda: MagicBlockSdk.delegationMetadataPdaFromDelegatedAccount(privateState),
      pda: privateState,
      payer: adminKeypair.publicKey,
      ownerProgram: solanaProgram.programId,
      delegationProgram: MagicBlockSdk.DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      validator,
    };
    const tx = await solanaProgram.methods
      .delegatePda({ privateState: { kickstarter: kickstarterPda } })
      .accountsPartial(delegateAccounts)
      .signers([adminKeypair])
      .rpc();
    addLog(`Delegated! Tx: ${tx}`);
    const router = new MagicBlockSdk.ConnectionMagicRouter(ephemeralRpcUrl);
    while (!(await router.getDelegationStatus(privateState)).isDelegated) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      console.log("Waiting for delegation to be active...");
    }
    addLog(`Delegation is active`);

  });


  it("Fund private", async () => {
    // sleep for 10 sec
    await new Promise((resolve) => setTimeout(resolve, 10000));
    const privateStateInfo = await ephemeralProgramInvestor.provider.connection.getAccountInfo(privateState);
      const isDelegated = (await (new MagicBlockSdk.ConnectionMagicRouter(ephemeralRpcUrl, 'confirmed').getDelegationStatus(
        privateState.toBase58()
      ))).isDelegated;

    console.log('[executePrivateFund] privateState:', {
      address: privateState.toBase58(),
      exists: !!privateStateInfo,
      owner: privateStateInfo?.owner.toBase58(),
      programId: solanaProgram.programId.toBase58(),
      dataLen: privateStateInfo?.data.length,
      isOwnedByProgram: privateStateInfo?.owner.equals(solanaProgram.programId),
      isDelegatedToRollup: isDelegated
    });


    addLog(`Private funding ${privateFundAmount} USDC from Investor ${investor}...`);

    const salt = crypto.getRandomValues(new Uint8Array(32));
    if (!(salt instanceof Uint8Array)) {
      throw new Error(`Salt must be Uint8Array, got ${typeof salt}`);
    }
    if (salt.length !== 32) {
      throw new Error(`Salt must be 32 bytes, got ${salt.length}`);
    }
    const saltArray = new Array(32);
    for (let i = 0; i < 32; i++) {
      saltArray[i] = salt[i];
    }

    const tx = await ephemeralProgramInvestor.methods
      .fundPrivate(new BN(amount), saltArray)
      .accounts({
        funder: investor,
        // @ts-ignore
        kickstarter: kickstarterPda,
        privateState,
        magicContext: MAGIC_CONTEXT_ID,
        magicProgram: MagicBlockSdk.MAGIC_PROGRAM_ID,
      })
      .signers([investorKeypair])
      .rpc();
    addLog(`Private funded! Tx: ${tx}`);
  });
});
