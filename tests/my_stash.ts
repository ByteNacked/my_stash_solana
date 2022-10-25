import * as anchor from "@project-serum/anchor";
import { BN, Program } from "@project-serum/anchor";
import { MyStash } from "../target/types/my_stash";
import { PublicKey, SystemProgram, Transaction, Connection, Commitment } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { assert } from "chai";

describe("my_stash", () => {
  // Configure the client to use the local cluster.
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MyStash as Program<MyStash>;

  let mint = null as Token;
  let tempTokenAccount = null;
  let recieverTokenAccount = null;
  let stashAuthorityPda = null;

  const mainAccount = anchor.web3.Keypair.generate();
  const stashStateAccount = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();

  const totalAmount = 10;
  const lockSeconds = 1;

  it("Initialize program state", async () => {
    // Airdropping tokens to a payer.
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 1000000000),
      "processed"
    );

    // Fund Main Accounts
    await provider.sendAndConfirm(
      (() => {
        const tx = new Transaction();
        tx.add(
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: mainAccount.publicKey,
            lamports: 100000000,
          }),
        );
        return tx;
      })(),
      [payer]
    );

    mint = await Token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );

    tempTokenAccount = await mint.createAccount(mainAccount.publicKey);
    
    await mint.mintTo(
      tempTokenAccount,
      mintAuthority.publicKey,
      [mintAuthority],
      totalAmount
    );

    let _initializerTokenAccount = await mint.getAccountInfo(tempTokenAccount);
    assert.ok(_initializerTokenAccount.amount.toNumber() == totalAmount);

    recieverTokenAccount = await mint.createAccount(mainAccount.publicKey);

  });

  it("Put to my_stash", async () => {
    await program.methods.initialize(new BN(lockSeconds))
        .accounts({
            initializer: mainAccount.publicKey,
            stashAccount: stashStateAccount.publicKey,
            stashTokenAccount: tempTokenAccount,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([mainAccount, stashStateAccount]).rpc();

    const [_stash_authority_pda, _stash_authority_bump] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("my_stash"))],
      program.programId
    );
    stashAuthorityPda = _stash_authority_pda;

    let _stash = await program.account.stashAccount.fetch(
      stashStateAccount.publicKey
    );

    // Check that the new owner is the PDA.
    let _stash_token = await mint.getAccountInfo(_stash.stashTokenAccount);
    assert.ok(_stash_token.owner.equals(stashAuthorityPda));

    // Check that the values in the stash account match what we expect.
    assert.ok(_stash.initializerKey.equals(mainAccount.publicKey));
    assert.ok(_stash.stashTokenAccount.equals(tempTokenAccount));
  });

  it("Retrive tokens", async () => {
    // Poor man's time forwarding ..
    function delay(milliseconds : number) {
        return new Promise(resolve => setTimeout(resolve, milliseconds));
    }
    await delay(lockSeconds * 1000).then(() => console.log('Waiting for ', lockSeconds, 'lock seconds') );

    await program.methods.retrieve()
        .accounts({
            initializer: mainAccount.publicKey,
            stashAccount: stashStateAccount.publicKey,
            stashTokenAccount: tempTokenAccount,
            stashTokenAccountAuthority: stashAuthorityPda,
            recieverTokenAccount: recieverTokenAccount,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([mainAccount]).rpc();

    // Check that the reciver get right token amount
    let _recieverTokenAccount = await mint.getAccountInfo(recieverTokenAccount);
    assert.ok(_recieverTokenAccount.amount.toNumber() == totalAmount);
  });
});

