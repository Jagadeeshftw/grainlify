/**
 * Failed Soroban transactions must surface their result code. The SDK
 * examples retry on `txBadSeq`, so the code needs to be in the error message.
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
  xdr,
} from "@stellar/stellar-sdk";
import { ProgramEscrowClient } from "../program-escrow-client";

const CONTRACT_ID = StrKey.encodeContract(Buffer.alloc(32, 9));
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
let client: ProgramEscrowClient;
let keypair: Keypair;

function txBadSeqResult(): xdr.TransactionResult {
  // A TransactionResult whose result code is txBadSeq.
  return xdr.TransactionResult.fromXDR("AAAAAAAAAGT////7AAAAAA==", "base64");
}

beforeEach(() => {
  keypair = Keypair.random();
  server = {
    getAccount: jest
      .fn()
      .mockResolvedValue(new Account(keypair.publicKey(), "1")),
    simulateTransaction: jest.fn().mockResolvedValue({
      result: { retval: xdr.ScVal.scvVoid() },
    }),
    sendTransaction: jest.fn(),
    getTransaction: jest.fn(),
  };

  ServerCtor.mockImplementation(() => server);
  assembleTransaction.mockImplementation((transaction: any) => ({
    build: () => transaction,
  }));

  client = new ProgramEscrowClient({
    contractId: CONTRACT_ID,
    rpcUrl: RPC_URL,
    networkPassphrase: PASSPHRASE,
    sourceKeypair: keypair,
  });
});

afterEach(() => {
  jest.clearAllMocks();
});

describe("transaction result codes", () => {
  it("includes the result code when submission returns ERROR", async () => {
    server.sendTransaction.mockResolvedValueOnce({
      status: "ERROR",
      hash: "abc123",
      errorResult: txBadSeqResult(),
    });

    await expect(client.lockProgramFunds(1n, keypair)).rejects.toThrow(
      "Soroban submission failed for lock_program_funds (txBadSeq)",
    );
  });

  it("includes the result code when polling reports FAILED", async () => {
    server.sendTransaction.mockResolvedValueOnce({
      status: "PENDING",
      hash: "abc123",
    });
    server.getTransaction.mockResolvedValueOnce({
      status: SorobanRpc.Api.GetTransactionStatus.FAILED,
      resultXdr: txBadSeqResult(),
    });

    await expect(client.lockProgramFunds(1n, keypair)).rejects.toThrow(
      "Soroban transaction failed for lock_program_funds (txBadSeq)",
    );
    expect(server.getTransaction).toHaveBeenCalledWith("abc123");
  });
});
