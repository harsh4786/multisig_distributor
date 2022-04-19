import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { MultisigDistributor } from "../target/types/multisig_distributor";

describe("multisig_distributor", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.MultisigDistributor as Program<MultisigDistributor>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
