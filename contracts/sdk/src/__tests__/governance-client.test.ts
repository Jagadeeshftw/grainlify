import {
  Account,
  Keypair,
  nativeToScVal,
  StrKey,
} from "@stellar/stellar-sdk";
import {
  ContractError,
  GovernanceClient,
  UpgradeProposal,
  ValidationError,
} from "..";

const CONTRACT_ID = StrKey.encodeContract(Buffer.alloc(32, 1));
const RPC_URL = "https://soroban-testnet.stellar.org";
const PASSPHRASE = "Test SDF Network ; September 2015";

function makeClient() {
  const client = new GovernanceClient({
    contractId: CONTRACT_ID,
    rpcUrl: RPC_URL,
    networkPassphrase: PASSPHRASE,
    sourceKeypair: Keypair.random(),
  });
  const invoke = jest.fn();
  (client as any).invokeContract = invoke;
  return { client, invoke };
}

function makeProposal(overrides: Partial<UpgradeProposal> = {}): UpgradeProposal {
  return {
    proposal_id: 7n,
    proposer: Keypair.random().publicKey(),
    wasm_hash: Buffer.alloc(32, 1),
    expiry: 2_000_000_000n,
    cancelled: false,
    ...overrides,
  };
}

describe("GovernanceClient", () => {
  it("encodes 32-byte wasm hashes from buffers and hex", async () => {
    const { client, invoke } = makeClient();
    invoke.mockResolvedValue(3n);
    const hash = Buffer.alloc(32, 0xab);

    await expect(
      client.proposeUpgrade(Keypair.random().publicKey(), hash, 0n),
    ).resolves.toBe(3n);
    await expect(
      client.proposeUpgrade(
        Keypair.random().publicKey(),
        `0x${hash.toString("hex")}`,
        0n,
      ),
    ).resolves.toBe(3n);

    expect(invoke.mock.calls[0][0]).toBe("propose_upgrade");
    expect(invoke.mock.calls[0][1][1]).toEqual(hash);
    expect(invoke.mock.calls[1][1][1]).toEqual(hash);
  });

  it.each([
    ["short buffer", Buffer.alloc(31)],
    ["long buffer", Buffer.alloc(33)],
    ["invalid hex", "xyz"],
    ["short hex", "ab".repeat(31)],
  ])("rejects a malformed wasm hash (%s)", async (_label, hash) => {
    const { client, invoke } = makeClient();
    await expect(
      client.proposeUpgrade(
        Keypair.random().publicKey(),
        hash as Buffer | string,
        0n,
      ),
    ).rejects.toBeInstanceOf(ValidationError);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("maps numeric contract failures through the governance error map", async () => {
    const { client, invoke } = makeClient();
    invoke.mockRejectedValue(
      new Error("HostError: Error(Contract, #102)"),
    );

    await expect(client.executeUpgrade(7n)).rejects.toMatchObject({
      code: "PAUSED",
      contractErrorCode: 102,
    });
    await expect(client.executeUpgrade(7n)).rejects.toBeInstanceOf(
      ContractError,
    );
  });

  it("covers the upgrade flow methods and converts proposal records", async () => {
    const { client, invoke } = makeClient();
    const proposal = makeProposal();
    invoke
      .mockResolvedValueOnce(7n)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(proposal)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(0n);

    await expect(
      client.proposeUpgrade(proposal.proposer!, proposal.wasm_hash, proposal.expiry),
    ).resolves.toBe(7n);
    await client.approveUpgrade(7n, proposal.proposer!);
    await client.cancelUpgrade(7n, proposal.proposer!);
    await expect(client.getUpgradeProposal(7n)).resolves.toEqual(proposal);
    await client.executeUpgrade(7n);
    await expect(client.getTimelockStatus(7n)).resolves.toBe(0n);

    expect(invoke.mock.calls.map(([method]) => method)).toEqual([
      "propose_upgrade",
      "approve_upgrade",
      "cancel_upgrade",
      "get_upgrade_proposal",
      "execute_upgrade",
      "get_timelock_status",
    ]);
  });

  it("uses a mocked Soroban RPC server for read-only simulations", async () => {
    const sourceKeypair = Keypair.random();
    const client = new GovernanceClient({
      contractId: CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: PASSPHRASE,
      sourceKeypair,
    });
    const server = (client as any).server;
    jest
      .spyOn(server, "getAccount")
      .mockResolvedValue(new Account(sourceKeypair.publicKey(), "1"));
    jest.spyOn(server, "simulateTransaction").mockResolvedValue({
      result: { retval: nativeToScVal(12, { type: "u32" }) },
    });

    await expect(client.getVersion()).resolves.toBe(12);
    expect(server.getAccount).toHaveBeenCalledWith(sourceKeypair.publicKey());
    expect(server.simulateTransaction).toHaveBeenCalledTimes(1);
  });

  describe("canExecute", () => {
    it("allows execution at the timelock boundary before proposal expiry", async () => {
      const { client, invoke } = makeClient();
      const proposal = makeProposal({ expiry: 2_000_000_000n });
      invoke.mockImplementation(async (method: string) => {
        if (method === "get_upgrade_proposal") return proposal;
        if (method === "get_timelock_status") return 0n;
        throw new Error(`Unexpected invocation: ${method}`);
      });
      jest
        .spyOn((client as any).server, "getLatestLedger")
        .mockResolvedValue({ closeTime: "1999999999" });

      await expect(client.canExecute(7n)).resolves.toBe(true);
    });

    it("does not allow execution one second before the timelock boundary", async () => {
      const { client, invoke } = makeClient();
      invoke.mockImplementation(async (method: string) => {
        if (method === "get_upgrade_proposal") return makeProposal();
        if (method === "get_timelock_status") return 1n;
        throw new Error(`Unexpected invocation: ${method}`);
      });

      await expect(client.canExecute(7n)).resolves.toBe(false);
    });

    it("does not allow execution exactly at proposal expiry", async () => {
      const { client, invoke } = makeClient();
      const proposal = makeProposal({ expiry: 2_000_000_000n });
      invoke.mockImplementation(async (method: string) => {
        if (method === "get_upgrade_proposal") return proposal;
        if (method === "get_timelock_status") return 0n;
        throw new Error(`Unexpected invocation: ${method}`);
      });
      jest
        .spyOn((client as any).server, "getLatestLedger")
        .mockResolvedValue({ closeTime: "2000000000" });

      await expect(client.canExecute(7n)).resolves.toBe(false);
    });

    it("does not query the ledger for missing or cancelled proposals", async () => {
      const { client, invoke } = makeClient();
      invoke.mockResolvedValueOnce(null).mockResolvedValueOnce(
        makeProposal({ cancelled: true }),
      );
      const latestLedger = jest.spyOn((client as any).server, "getLatestLedger");

      await expect(client.canExecute(7n)).resolves.toBe(false);
      await expect(client.canExecute(7n)).resolves.toBe(false);
      expect(latestLedger).not.toHaveBeenCalled();
    });
  });

  it("validates migration hashes and maps pause/liveness methods", async () => {
    const { client, invoke } = makeClient();
    invoke
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(4)
      .mockResolvedValueOnce(true)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined);
    const signer = Keypair.random().publicKey();

    await client.commitMigration(4, Buffer.alloc(32), 0n);
    await client.migrate(4, "01".repeat(32));
    await expect(client.migrate(4, Buffer.alloc(31))).rejects.toBeInstanceOf(
      ValidationError,
    );
    await expect(client.getVersion()).resolves.toBe(4);
    await expect(client.isPaused()).resolves.toBe(true);
    await client.pause(signer);
    await client.unpause(signer);

    expect(invoke.mock.calls.map(([method]) => method)).toEqual([
      "commit_migration",
      "migrate",
      "get_version",
      "is_paused",
      "pause",
      "unpause",
    ]);
  });
});
