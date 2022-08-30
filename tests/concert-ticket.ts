import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { ConcertTicket } from "../target/types/concert_ticket";
import {LAMPORTS_PER_SOL} from '@solana/web3.js';
import { assert } from "chai";
import * as spl from '@solana/spl-token';

describe("concert-ticket", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ConcertTicket as Program<ConcertTicket>;

  const alice = anchor.web3.Keypair.generate();
  const bob = anchor.web3.Keypair.generate();

  let tokenMint: anchor.web3.PublicKey;
  let aliceTokenAccount: anchor.web3.PublicKey;
  let bobTokenAccount: anchor.web3.PublicKey;

  it("Is wallet funded", async () => {
    // Add your test here.

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        alice.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    )

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        bob.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    )

    const balance = await provider.connection.getBalance(alice.publicKey);
    assert.equal(balance, 2 * LAMPORTS_PER_SOL);

  });

  it("mint some tokens", async() => {

    tokenMint = await spl.createMint(
      provider.connection,
      alice,
      alice.publicKey,
      alice.publicKey,
      6,
    )

    aliceTokenAccount = await spl.createAssociatedTokenAccount(
      provider.connection,
      alice,
      tokenMint,
      alice.publicKey
    )

    bobTokenAccount = await spl.createAssociatedTokenAccount(
      provider.connection,
      bob,
      tokenMint,
      bob.publicKey
    )

    await spl.mintTo(
      provider.connection,
      bob,
      tokenMint,
      bobTokenAccount,
      alice,
      1000000
    );

  })

  let venueId = "123";
  it("create venue", async() => {

    const [venuePDA, venueBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("venue"), Buffer.from(venueId)],
      program.programId
    )

    const tx = await program.methods.createVenue(venueId).accounts({
      venueAccount: venuePDA,
      authority: alice.publicKey,
      tokenMint: tokenMint,
      tokenAccount: aliceTokenAccount,
      systemProgram: anchor.web3.SystemProgram.programId,
    }).signers([alice]).rpc();

    const state = await program.account.venue.fetch(venuePDA);

    assert.equal(state.owner.toBase58(), alice.publicKey.toBase58())

  })

  it("create tickets", async() => {
    const [venuePDA, venueBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("venue"), Buffer.from(venueId)],
      program.programId
    )

    const tx = await program.methods.createTickets(venueId, venueBump, "basic", new anchor.BN(1000), new anchor.BN(10)).accounts({
     venueAccount: venuePDA,
     owner: alice.publicKey
    }).signers([alice]).rpc();

    const state = await program.account.venue.fetch(venuePDA)

    assert.equal(state.availableTickets[0].name, "basic");

  })

  it("purchase tickets", async() => {
    const [venuePDA, venueBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("venue"), Buffer.from(venueId)],
      program.programId
    )
    const [ticketPDA, ticketBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("ticket"), Buffer.from(venueId), bob.publicKey.toBuffer()],
      program.programId
    )

    const tx = await program.methods.purchaseTickets(venueId, venueBump, "basic", new anchor.BN(8)).accounts({
      venueAccount: venuePDA,
      buyerAccount: ticketPDA,
      buyer: bob.publicKey,
      buyerTokenAccount: bobTokenAccount,
      venueOwnerTokenAccount: aliceTokenAccount,
      tokenProgram: spl.TOKEN_PROGRAM_ID,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      systemProgram: anchor.web3.SystemProgram.programId
    }).signers([bob]).rpc();


    const aliceBalance = await spl.getAccount(provider.connection, aliceTokenAccount);
    assert.equal(aliceBalance.amount.toString(), "1000");


  })

});
