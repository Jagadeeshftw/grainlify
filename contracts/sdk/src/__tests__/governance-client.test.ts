import {
  Account,
  Keypair,
  nativeToScVal,
  SorobanRpc,
  StrKey,
} from "@stellar/stellar-sdk";
import { ContractError, ValidationError, ContractErrorCode } from "../errors";
import { GovernanceClient } from "../governance-client";

const CONTRACT_ID = StrKey.encodeContract(Buffer.alloc(32, 1));

describe("GovernanceClient", () => {
  let client: GovernanceClient;
  let sourceKeypair: Keypair;

  beforeEach(() => {
    sourceKeypair = Keypair.random();
    client = new GovernanceClient({
      contractId: CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
      sourceKeypair,
    });
  });

  it("uses the mocked Soroban RPC server for read-only invocations", async () => {
    const server = (
      client as unknown as { server: SorobanRpc.Server }
    ).server;
    jest
      .spyOn(server, "getAccount")
      .mockResolvedValue(new Account(sourceKeypair.publicKey(), "0"));
    const simulate = jest
      .spyOn(server, "simulateTransaction")
      .mockResolvedValue({
        result: { retval: nativeToScVal(7, { type: "u32" }) },
      } as Awaited<ReturnType<SorobanRpc.Server["simulateTransaction"]>>);

    await expect(client.getVersion()).resolves.toBe(7);
    expect(simulate).toHaveBeenCalledTimes(1);
  });

  it("rejects hashes that are not exactly 32 bytes before RPC invocation", async () => {
    await expect(
      client.proposeUpgrade(
        sourceKeypair.publicKey(),
        Buffer.alloc(31),
        0n,
        sourceKeypair,
      ),
    ).rejects.toBeInstanceOf(ValidationError);
  });

  it("accepts a 32-byte hexadecimal hash", async () => {
    const internal = client as unknown as {
      invokeContract: (...args: unknown[]) => Promise<unknown>;
    };
    const invoke = jest
      .spyOn(internal, "invokeContract")
      .mockResolvedValue(12n);

    await expect(
      client.proposeUpgrade(
        sourceKeypair.publicKey(),
        "01".repeat(32),
        0n,
        sourceKeypair,
      ),
    ).resolves.toBe(12n);
    expect(invoke).toHaveBeenCalledWith(
      "propose_upgrade",
      [sourceKeypair.publicKey(), Buffer.alloc(32, 1), 0n],
      sourceKeypair,
    );
  });

  it("maps numeric contract failures to governance-specific typed errors", async () => {
    const internal = client as unknown as {
      invokeContract: (...args: unknown[]) => Promise<unknown>;
    };
    jest
      .spyOn(internal, "invokeContract")
      .mockRejectedValue(new Error("HostError: Error(Contract, 6)"));

    await expect(
      client.approveUpgrade(4n, sourceKeypair.publicKey(), sourceKeypair),
    ).rejects.toMatchObject({
      code: ContractErrorCode.GOV_PROPOSAL_NOT_FOUND,
      contractErrorCode: 6,
    } satisfies Partial<ContractError>);
  });

  it("reports readiness exactly when the timelock reaches zero", async () => {
    const internal = client as unknown as {
      invokeContract: (...args: unknown[]) => Promise<unknown>;
      getLatestLedgerTime: () => Promise<bigint>;
    };
    jest.spyOn(internal, "invokeContract").mockImplementation(async (method) => {
      if (method === "get_upgrade_proposal") {
        return [3n, sourceKeypair.publicKey(), Buffer.alloc(32), 101n, false];
      }
      if (method === "get_timelock_status") return 0n;
      throw new Error(`Unexpected contract method: ${method}`);
    });
    jest
      .spyOn(internal, "getLatestLedgerTime")
      .mockResolvedValue(100n);

    await expect(client.getTimelockStatus(3n)).resolves.toEqual({
      remainingSeconds: 0n,
      ready: true,
    });
    await expect(client.canExecute(3n)).resolves.toBe(true);
  });

  it("does not allow execution at the exact proposal expiry boundary", async () => {
    const internal = client as unknown as {
      invokeContract: (...args: unknown[]) => Promise<unknown>;
      getLatestLedgerTime: () => Promise<bigint>;
    };
    jest.spyOn(internal, "invokeContract").mockImplementation(async (method) => {
      if (method === "get_upgrade_proposal") {
        return [3n, null, Buffer.alloc(32), 100n, false];
      }
      if (method === "get_timelock_status") return 0n;
      throw new Error(`Unexpected contract method: ${method}`);
    });
    jest
      .spyOn(internal, "getLatestLedgerTime")
      .mockResolvedValue(100n);

    await expect(client.canExecute(3n)).resolves.toBe(false);
  });
});
