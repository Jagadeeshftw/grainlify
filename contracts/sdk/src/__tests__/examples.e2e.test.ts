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

const rawAttempts = Number(process.env.GRAINLIFY_SDK_E2E_ATTEMPTS ?? "3");
const maxAttempts =
  Number.isFinite(rawAttempts) && rawAttempts > 0 ? Math.floor(rawAttempts) : 3;
const rawRetryDelay = Number(
  process.env.GRAINLIFY_SDK_E2E_RETRY_DELAY_MS ?? "2000",
);
const retryDelayMs =
  Number.isFinite(rawRetryDelay) && rawRetryDelay >= 0 ? rawRetryDelay : 2000;

// Every attempt gets a fresh suffix so a retried run never collides with a
// program created by the previous attempt.
const runId = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const describeDeployment = configured || required ? describe : describe.skip;

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isTxBadSeq(error: unknown): boolean {
  return /txBadSeq|tx_bad_seq/i.test(errorMessage(error));
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describeDeployment("SDK examples against a deployed Soroban contract", () => {
  let signer: Keypair;

  beforeAll(() => {
    if (!configured) {
      throw new Error(
        "The dedicated E2E job must set every GRAINLIFY_SDK_E2E_* value",
      );
    }
    signer = Keypair.fromSecret(secret!);
  });

  function makeClient(): ProgramEscrowClient {
    return new ProgramEscrowClient({
      contractId: contractId!,
      rpcUrl: rpcUrl!,
      networkPassphrase: networkPassphrase!,
      sourceKeypair: signer,
    });
  }

  async function runExample<T>(
    client: ProgramEscrowClient,
    name: string,
    execute: () => Promise<T>,
  ): Promise<T> {
    try {
      return await execute();
    } catch (error) {
      const reason = errorMessage(error);
      throw new Error(
        `${name} failed against contract ${contractId}: ${reason}`,
      );
    }
  }

  it(
    "runs create, read, and state-changing examples against the real deployment",
    async () => {
      let lastError: unknown;

      for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        const programId = `sdk-e2e-${runId}-${attempt}`;
        const client = makeClient();

        try {
          const initialized = await runExample(
            client,
            "full-lifecycle example",
            () =>
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

          const queried = await runExample(client, "query-escrow example", () =>
            queryEscrowExample(client),
          );
          expect(queried.program_id).toBe(programId);

          const locked = await runExample(client, "lock-funds example", () =>
            lockFundsExample(client, signer),
          );
          expect(locked.program_id).toBe(programId);

          const batched = await runExample(client, "batch-lock example", () =>
            batchLockExample(client, signer),
          );
          expect(batched.program_id).toBe(programId);

          const released = await runExample(
            client,
            "release-funds example",
            () => releaseFundsExample(client, signer),
          );
          expect(Number.isInteger(released)).toBe(true);
          return;
        } catch (error) {
          lastError = error;
          if (!isTxBadSeq(error) || attempt === maxAttempts) {
            throw error;
          }

          console.warn(
            `[sdk-e2e] attempt ${attempt}/${maxAttempts} hit txBadSeq; ` +
              `retrying with program id sdk-e2e-${runId}-${attempt + 1}`,
          );
          await sleep(retryDelayMs * attempt);
        }
      }

      throw lastError;
    },
    180_000,
  );
});
