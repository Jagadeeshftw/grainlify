import { Keypair } from "@stellar/stellar-sdk";
import { batchLockExample } from "../../examples/batch-lock";
import { fullLifecycleExample } from "../../examples/full-lifecycle";
import { lockFundsExample } from "../../examples/lock-funds";
import { queryEscrowExample } from "../../examples/query-escrow";
import { releaseFundsExample } from "../../examples/release-funds";
import { ProgramEscrowClient } from "../program-escrow-client";

const contractId = process.env.GRAINLIFY_SDK_E2E_CONTRACT_ID;
const rpcUrl = process.env.GRAINLIFY_SDK_E2E_RPC_URL;
const networkPassphrase = process.env.GRAINLIFY_SDK_E2E_NETWORK_PASSPHRASE;
const secret = process.env.GRAINLIFY_SDK_E2E_SECRET;
const tokenAddress = process.env.GRAINLIFY_SDK_E2E_TOKEN_ADDRESS;
const configured = Boolean(
  contractId && rpcUrl && networkPassphrase && secret && tokenAddress,
);
const required = process.env.GRAINLIFY_SDK_E2E_REQUIRED === "1";

const describeDeployment = configured || required ? describe : describe.skip;

describeDeployment("SDK examples against a deployed Soroban contract", () => {
  let signer: Keypair;
  let client: ProgramEscrowClient;
  const programId = `sdk-e2e-${Date.now()}`;

  beforeAll(() => {
    if (!configured) {
      throw new Error("The dedicated E2E job must set every GRAINLIFY_SDK_E2E_* value");
    }
    signer = Keypair.fromSecret(secret!);
    client = new ProgramEscrowClient({
      contractId: contractId!,
      rpcUrl: rpcUrl!,
      networkPassphrase: networkPassphrase!,
      sourceKeypair: signer,
    });
  });

  async function runExample<T>(name: string, execute: () => Promise<T>): Promise<T> {
    try {
      return await execute();
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      throw new Error(`${name} failed against contract ${contractId}: ${reason}`);
    }
  }

  it("runs create, read, and state-changing examples against the real deployment", async () => {
    const initialized = await runExample("full-lifecycle example", () =>
      fullLifecycleExample(
        client,
        signer,
        programId,
        signer.publicKey(),
        tokenAddress!,
      ),
    );
    expect(initialized.program_id).toBe(programId);
    expect(initialized.total_funds).toBeGreaterThan(0n);

    const queried = await runExample("query-escrow example", () =>
      queryEscrowExample(client),
    );
    expect(queried.program_id).toBe(programId);

    const locked = await runExample("lock-funds example", () =>
      lockFundsExample(client, signer),
    );
    expect(locked.program_id).toBe(programId);

    const batched = await runExample("batch-lock example", () =>
      batchLockExample(client, signer),
    );
    expect(batched.program_id).toBe(programId);

    const released = await runExample("release-funds example", () =>
      releaseFundsExample(client, signer),
    );
    expect(Number.isInteger(released)).toBe(true);
  }, 120_000);
});
