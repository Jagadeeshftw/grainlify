/**
 * BountyEscrowClient tests.
 *
 * `SorobanRpc.Server` is mocked so no network calls are made. The tests cover
 * argument encoding, client-side validation, contract-error mapping and the
 * read-only simulation path (which must never submit a transaction).
 */
jest.mock("@stellar/stellar-sdk", () => {
  const actual = jest.requireActual("@stellar/stellar-sdk");
  return {
    ...actual,
    SorobanRpc: {
      ...actual.SorobanRpc,
      Server: jest.fn(),
      assembleTransaction: jest.fn(),
    },
  };
});

import {
  Account,
  Keypair,
  SorobanRpc,
  StrKey,
  nativeToScVal,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";
import { BountyEscrowClient } from "../bounty-escrow-client";
import {
  ContractError,
  ContractErrorCode,
  ValidationError,
} from "../errors";

const CONTRACT_ID = StrKey.encodeContract(Buffer.alloc(32, 2));
const VALID_ADDRESS = Keypair.random().publicKey();
const VALID_ADDRESS_2 = Keypair.random().publicKey();
const TOKEN_CONTRACT_ID = StrKey.encodeContract(Buffer.alloc(32, 1));
const RPC_URL = "https://soroban-testnet.stellar.org";
const PASSPHRASE = "Test SDF Network ; September 2015";

type MockServer = {
  getAccount: jest.Mock;
  simulateTransaction: jest.Mock;
  sendTransaction: jest.Mock;
  getTransaction: jest.Mock;
};

const ServerCtor = SorobanRpc.Server as unknown as jest.Mock;
const assembleTransaction =
  SorobanRpc.assembleTransaction as unknown as jest.Mock;

let server: MockServer;
let client: BountyEscrowClient;
let keypair: Keypair;

function makeClient(): BountyEscrowClient {
  return new BountyEscrowClient({
    contractId: CONTRACT_ID,
    rpcUrl: RPC_URL,
    networkPassphrase: PASSPHRASE,
    // Used by the read-only methods, which have no signer argument.
    sourceKeypair: keypair,
  });
}

function entry(key: string, value: xdr.ScVal): xdr.ScMapEntry {
  return new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol(key), val: value });
}

function escrowRetval(): xdr.ScVal {
  return xdr.ScVal.scvMap([
    entry(
      "depositor",
      nativeToScVal(VALID_ADDRESS, { type: "address" }),
    ),
    entry("amount", nativeToScVal(1000n, { type: "i128" })),
    entry("remaining_amount", nativeToScVal(400n, { type: "i128" })),
    entry("status", xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("Locked")])),
    entry("deadline", nativeToScVal(1_800_000_000n, { type: "u64" })),
    entry("refund_history", xdr.ScVal.scvVec([])),
    entry("archived", xdr.ScVal.scvBool(false)),
    entry("archived_at", xdr.ScVal.scvVoid()),
  ]);
}

function pauseFlagsRetval(): xdr.ScVal {
  return xdr.ScVal.scvMap([
    entry("lock_paused", xdr.ScVal.scvBool(true)),
    entry("release_paused", xdr.ScVal.scvBool(false)),
    entry("refund_paused", xdr.ScVal.scvBool(true)),
    entry("pause_reason", nativeToScVal("maintenance", { type: "string" })),
    entry("paused_at", nativeToScVal(1_700_000_000n, { type: "u64" })),
  ]);
}

function simulateSuccess(retval: xdr.ScVal): void {
  server.simulateTransaction.mockResolvedValueOnce({
    result: { retval },
  });
}

function simulateWriteSuccess(retval?: xdr.ScVal): void {
  simulateSuccess(retval ?? xdr.ScVal.scvVoid());
  server.getTransaction.mockResolvedValueOnce({
    status: SorobanRpc.Api.GetTransactionStatus.SUCCESS,
    returnValue: retval,
  });
}

function encode(method: string, args: any[]): xdr.ScVal[] {
  return (client as any).encodeArguments(method, args, keypair);
}

function nativeArgs(method: string, args: any[]): any[] {
  return encode(method, args).map((value) => scValToNative(value));
}

beforeEach(() => {
  keypair = Keypair.random();
  server = {
    getAccount: jest
      .fn()
      .mockResolvedValue(new Account(keypair.publicKey(), "1")),
    simulateTransaction: jest.fn(),
    sendTransaction: jest.fn().mockResolvedValue({
      status: "PENDING",
      hash: "abc123",
    }),
    getTransaction: jest.fn(),
  };

  ServerCtor.mockImplementation(() => server);
  assembleTransaction.mockImplementation((transaction: any) => ({
    build: () => transaction,
  }));

  client = makeClient();
});

afterEach(() => {
  jest.clearAllMocks();
});

// =======================================================================
// Encoding
// =======================================================================
describe("BountyEscrowClient argument encoding", () => {
  it("encodes init(admin, token)", () => {
    expect(nativeArgs("init", [VALID_ADDRESS, TOKEN_CONTRACT_ID])).toEqual([
      VALID_ADDRESS,
      TOKEN_CONTRACT_ID,
    ]);
  });

  it("encodes lock_funds with Address, u64 bounty id, i128 amount and u64 deadline", () => {
    expect(
      nativeArgs("lock_funds", [
        VALID_ADDRESS,
        7n,
        1_000n,
        1_800_000_000n,
      ]),
    ).toEqual([VALID_ADDRESS, 7n, 1_000n, 1_800_000_000n]);
  });

  it("encodes batch_lock_funds as a Vec of LockFundsItem structs", () => {
    const [items] = nativeArgs("batch_lock_funds", [
      [
        {
          bounty_id: 1n,
          depositor: VALID_ADDRESS,
          amount: 100n,
          deadline: 2_000_000_000n,
        },
        {
          bounty_id: 2n,
          depositor: VALID_ADDRESS_2,
          amount: 200n,
          deadline: 2_000_000_100n,
        },
      ],
    ]);

    expect(items).toEqual([
      {
        amount: 100n,
        bounty_id: 1n,
        deadline: 2_000_000_000n,
        depositor: VALID_ADDRESS,
      },
      {
        amount: 200n,
        bounty_id: 2n,
        deadline: 2_000_000_100n,
        depositor: VALID_ADDRESS_2,
      },
    ]);
  });

  it("encodes release_funds and partial_release", () => {
    expect(nativeArgs("release_funds", [3n, VALID_ADDRESS])).toEqual([
      3n,
      VALID_ADDRESS,
    ]);
    expect(
      nativeArgs("partial_release", [3n, VALID_ADDRESS, 250n]),
    ).toEqual([3n, VALID_ADDRESS, 250n]);
  });

  it("encodes approve_refund with the RefundMode enum", () => {
    const encoded = nativeArgs("approve_refund", [
      9n,
      500n,
      VALID_ADDRESS,
      "Full",
    ]);

    expect(encoded).toEqual([9n, 500n, VALID_ADDRESS, ["Full"]]);
  });

  it("encodes set_paused optionals as void when omitted", () => {
    expect(
      nativeArgs("set_paused", [true, undefined, false, "maintenance"]),
    ).toEqual([true, null, false, "maintenance"]);
  });

  it("rejects an unknown method with an explicit error", () => {
    expect(() => encode("not_a_method", [])).toThrow(
      "No Soroban argument encoder is defined for not_a_method",
    );
  });
});

// =======================================================================
// Validation
// =======================================================================
describe("BountyEscrowClient validation", () => {
  it("rejects non-positive lock amounts before calling the network", async () => {
    await expect(
      client.lockFunds(VALID_ADDRESS, 1n, 0n, 1_800_000_000n, keypair),
    ).rejects.toBeInstanceOf(ValidationError);
    await expect(
      client.lockFunds(VALID_ADDRESS, 1n, -5n, 1_800_000_000n, keypair),
    ).rejects.toBeInstanceOf(ValidationError);
    expect(server.getAccount).not.toHaveBeenCalled();
    expect(server.simulateTransaction).not.toHaveBeenCalled();
  });

  it("rejects an invalid depositor address", async () => {
    await expect(
      client.lockFunds("not-a-stellar-address", 1n, 100n, 1_800_000_000n, keypair),
    ).rejects.toMatchObject({
      name: "ValidationError",
      field: "depositor",
    });
  });

  it("rejects an invalid token contract address in init", async () => {
    await expect(
      client.init(VALID_ADDRESS, "not-a-contract", keypair),
    ).rejects.toMatchObject({
      name: "ValidationError",
      field: "token",
    });
  });

  it("rejects empty and duplicate batches before calling the network", async () => {
    await expect(client.batchLockFunds([], keypair)).rejects.toBeInstanceOf(
      ValidationError,
    );

    await expect(
      client.batchLockFunds(
        [
          {
            bounty_id: 1n,
            depositor: VALID_ADDRESS,
            amount: 100n,
            deadline: 1_800_000_000n,
          },
          {
            bounty_id: 1n,
            depositor: VALID_ADDRESS_2,
            amount: 200n,
            deadline: 1_800_000_000n,
          },
        ],
        keypair,
      ),
    ).rejects.toBeInstanceOf(ValidationError);

    expect(server.getAccount).not.toHaveBeenCalled();
  });

  it("rejects a non-positive partial release amount", async () => {
    await expect(
      client.partialRelease(1n, VALID_ADDRESS, 0n, keypair),
    ).rejects.toBeInstanceOf(ValidationError);
  });

  it("rejects negative bounty ids", async () => {
    await expect(client.getEscrowInfo(-1n)).rejects.toBeInstanceOf(
      ValidationError,
    );
  });

  it("rejects an invalid refund mode", async () => {
    await expect(
      client.approveRefund(
        1n,
        100n,
        VALID_ADDRESS,
        "Sometimes" as any,
        keypair,
      ),
    ).rejects.toBeInstanceOf(ValidationError);
  });
});

// =======================================================================
// Write path
// =======================================================================
describe("BountyEscrowClient write path", () => {
  it("submits lock_funds and waits for confirmation", async () => {
    simulateWriteSuccess();

    await client.lockFunds(
      VALID_ADDRESS,
      1n,
      1_000n,
      1_800_000_000n,
      keypair,
    );

    expect(server.simulateTransaction).toHaveBeenCalledTimes(1);
    expect(server.sendTransaction).toHaveBeenCalledTimes(1);
    expect(server.getTransaction).toHaveBeenCalledWith("abc123");
  });

  it("returns the created count from batch_lock_funds", async () => {
    simulateWriteSuccess(nativeToScVal(2, { type: "u32" }));

    const created = await client.batchLockFunds(
      [
        {
          bounty_id: 1n,
          depositor: VALID_ADDRESS,
          amount: 100n,
          deadline: 1_800_000_000n,
        },
        {
          bounty_id: 2n,
          depositor: VALID_ADDRESS_2,
          amount: 200n,
          deadline: 1_800_000_000n,
        },
      ],
      keypair,
    );

    expect(created).toBe(2);
  });

  it("submits approve_refund", async () => {
    simulateWriteSuccess();

    await client.approveRefund(1n, 500n, VALID_ADDRESS, "Partial", keypair);

    expect(server.sendTransaction).toHaveBeenCalledTimes(1);
  });
});

// =======================================================================
// Read-only simulation path
// =======================================================================
describe("BountyEscrowClient read-only methods", () => {
  it("getEscrowInfo simulates without submitting", async () => {
    simulateSuccess(escrowRetval());

    const escrow = await client.getEscrowInfo(1n);

    expect(server.simulateTransaction).toHaveBeenCalledTimes(1);
    expect(server.sendTransaction).not.toHaveBeenCalled();
    expect(server.getTransaction).not.toHaveBeenCalled();
    expect(escrow).toEqual({
      depositor: VALID_ADDRESS,
      amount: 1_000n,
      remaining_amount: 400n,
      status: "Locked",
      deadline: 1_800_000_000,
      refund_history: [],
      archived: false,
      archived_at: undefined,
    });
  });

  it("getBalance simulates without submitting", async () => {
    simulateSuccess(nativeToScVal(5_000n, { type: "i128" }));

    const balance = await client.getBalance();

    expect(balance).toBe(5_000n);
    expect(server.sendTransaction).not.toHaveBeenCalled();
  });

  it("getRefundEligibility maps the contract tuple", async () => {
    simulateSuccess(
      xdr.ScVal.scvVec([
        xdr.ScVal.scvBool(true),
        xdr.ScVal.scvBool(false),
        nativeToScVal(750n, { type: "i128" }),
        xdr.ScVal.scvVoid(),
      ]),
    );

    const eligibility = await client.getRefundEligibility(4n);

    expect(eligibility).toEqual({
      canRefund: true,
      deadlinePassed: false,
      remainingAmount: 750n,
      approval: null,
    });
    expect(server.sendTransaction).not.toHaveBeenCalled();
  });

  it("getPauseFlags simulates without submitting", async () => {
    simulateSuccess(pauseFlagsRetval());

    const flags = await client.getPauseFlags();

    expect(flags).toEqual({
      lock_paused: true,
      release_paused: false,
      refund_paused: true,
      pause_reason: "maintenance",
      paused_at: 1_700_000_000,
    });
    expect(server.sendTransaction).not.toHaveBeenCalled();
  });
});

// =======================================================================
// Contract error mapping
// =======================================================================
describe("BountyEscrowClient contract error mapping", () => {
  it("maps numeric bounty-escrow codes with parseContractErrorByCode", async () => {
    server.simulateTransaction.mockResolvedValueOnce({
      error: "HostError: Error(Contract, 202)",
    });

    try {
      await client.getEscrowInfo(99n);
      fail("Expected ContractError");
    } catch (error: any) {
      expect(error).toBeInstanceOf(ContractError);
      expect(error.code).toBe(ContractErrorCode.BOUNTY_NOT_FOUND);
      expect(error.contractErrorCode).toBe(202);
    }
  });

  it("maps numeric codes for write methods too", async () => {
    server.simulateTransaction.mockResolvedValueOnce({
      error: "HostError: Error(Contract, 17)",
    });

    await expect(
      client.refund(1n, keypair),
    ).rejects.toMatchObject({
      code: ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED,
      contractErrorCode: 17,
    });
  });

  it("maps plain contract panic strings", async () => {
    server.simulateTransaction.mockResolvedValueOnce({
      error: "HostError: FundsNotLocked",
    });

    await expect(client.releaseFunds(1n, VALID_ADDRESS, keypair)).rejects.toBeInstanceOf(
      ContractError,
    );
  });
});
