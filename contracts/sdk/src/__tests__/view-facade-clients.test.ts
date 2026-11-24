import {
  Address,
  nativeToScVal,
  SorobanRpc,
  xdr,
} from "@stellar/stellar-sdk";
import {
  ContractError,
  ContractKind,
  DEFAULT_SIMULATION_SOURCE_ACCOUNT,
  EscrowStatus,
  EscrowViewFacadeClient,
  NetworkError,
  NotFoundError,
  ValidationError,
  ViewFacadeClient,
} from "../index";

const NETWORK_PASSPHRASE = "Test SDF Network ; September 2015";
const RPC_URL = "https://soroban-testnet.stellar.org";

// Valid StrKey contract & account addresses for XDR encoding in tests
const FACADE_CONTRACT_ID =
  "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4";
const ESCROW_CONTRACT_ID =
  "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM";
const PROGRAM_CONTRACT_ID =
  "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFCT4";
const TOKEN_CONTRACT_ID =
  "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHK3M";
const USER_ADDRESS =
  "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const DEPOSITOR_ADDRESS =
  "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ";
const CUSTOM_SOURCE_ACCOUNT =
  "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ";

function okSimulation(retval: xdr.ScVal): SorobanRpc.Api.SimulateTransactionSuccessResponse {
  return {
    id: "sim-1",
    latestLedger: 12345,
    cost: { cpuInsns: "100", memBytes: "100" },
    minResourceFee: "100",
    transactionData: {} as any,
    result: {
      auth: [],
      retval,
    },
  } as unknown as SorobanRpc.Api.SimulateTransactionSuccessResponse;
}

function errorSimulation(error: string): SorobanRpc.Api.SimulateTransactionErrorResponse {
  return {
    id: "sim-err",
    latestLedger: 12345,
    error,
  } as unknown as SorobanRpc.Api.SimulateTransactionErrorResponse;
}

function makeMockServer() {
  return {
    simulateTransaction: jest.fn(),
    getAccount: jest.fn(),
    sendTransaction: jest.fn(),
  } as unknown as SorobanRpc.Server & {
    simulateTransaction: jest.Mock;
    getAccount: jest.Mock;
    sendTransaction: jest.Mock;
  };
}

function scMap(entries: Array<[string, xdr.ScVal]>): xdr.ScVal {
  return xdr.ScVal.scvMap(
    entries.map(
      ([k, v]) =>
        new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol(k),
          val: v,
        }),
    ),
  );
}

function makeRegisteredContractScVal(
  address: string,
  kind: ContractKind,
  version: number,
): xdr.ScVal {
  return scMap([
    ["address", Address.fromString(address).toScVal()],
    ["kind", xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(kind)])],
    ["version", nativeToScVal(version, { type: "u32" })],
  ]);
}

function makeEscrowSummaryScVal(overrides?: {
  bounty_id?: bigint;
  depositor?: string;
  amount?: bigint;
  remaining_amount?: bigint;
  status?: EscrowStatus;
  deadline?: bigint;
  repo_id?: bigint;
  issue_id?: bigint;
  bounty_type?: string;
  is_paused?: boolean;
}): xdr.ScVal {
  const s = {
    bounty_id: 42n,
    depositor: DEPOSITOR_ADDRESS,
    amount: 5_000_000_000n,
    remaining_amount: 2_500_000_000n,
    status: EscrowStatus.Locked,
    deadline: 1_800_000_000n,
    repo_id: 101n,
    issue_id: 202n,
    bounty_type: "feature",
    is_paused: false,
    ...overrides,
  };
  return scMap([
    ["bounty_id", nativeToScVal(s.bounty_id, { type: "u64" })],
    ["depositor", Address.fromString(s.depositor).toScVal()],
    ["amount", nativeToScVal(s.amount, { type: "i128" })],
    ["remaining_amount", nativeToScVal(s.remaining_amount, { type: "i128" })],
    ["status", xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(s.status)])],
    ["deadline", nativeToScVal(s.deadline, { type: "u64" })],
    ["repo_id", nativeToScVal(s.repo_id, { type: "u64" })],
    ["issue_id", nativeToScVal(s.issue_id, { type: "u64" })],
    ["bounty_type", nativeToScVal(s.bounty_type, { type: "string" })],
    ["is_paused", xdr.ScVal.scvBool(s.is_paused)],
  ]);
}

function makeProgramDataScVal(programId = "prog-1"): xdr.ScVal {
  return scMap([
    ["program_id", nativeToScVal(programId, { type: "string" })],
    ["total_funds", nativeToScVal(10_000_000_000n, { type: "i128" })],
    ["remaining_balance", nativeToScVal(7_500_000_000n, { type: "i128" })],
    ["authorized_payout_key", Address.fromString(DEPOSITOR_ADDRESS).toScVal()],
    [
      "payout_history",
      xdr.ScVal.scvVec([
        scMap([
          ["recipient", Address.fromString(USER_ADDRESS).toScVal()],
          ["amount", nativeToScVal(2_500_000_000n, { type: "i128" })],
          ["timestamp", nativeToScVal(1_700_000_123n, { type: "u64" })],
        ]),
      ]),
    ],
    ["token_address", Address.fromString(TOKEN_CONTRACT_ID).toScVal()],
    ["risk_flags", nativeToScVal(0, { type: "u32" })],
    ["initial_liquidity", nativeToScVal(1_000_000_000n, { type: "i128" })],
  ]);
}

function makeFeeConfigScVal(): xdr.ScVal {
  return scMap([
    ["lock_fee_rate", nativeToScVal(100n, { type: "i128" })],
    ["payout_fee_rate", nativeToScVal(50n, { type: "i128" })],
    ["lock_fixed_fee", nativeToScVal(1_000n, { type: "i128" })],
    ["payout_fixed_fee", nativeToScVal(500n, { type: "i128" })],
    ["fee_recipient", Address.fromString(DEPOSITOR_ADDRESS).toScVal()],
    ["fee_enabled", xdr.ScVal.scvBool(true)],
    ["fee_waivers", nativeToScVal(1, { type: "u32" })],
    ["insurance_reserve_bps", nativeToScVal(250, { type: "u32" })],
  ]);
}

// ---------------------------------------------------------------------------
// ViewFacadeClient
// ---------------------------------------------------------------------------

describe("ViewFacadeClient", () => {
  it("simulates without requiring a signing keypair and uses default or custom sourceAccount", async () => {
    const server = makeMockServer();
    server.simulateTransaction.mockResolvedValue(
      okSimulation(nativeToScVal(3, { type: "u32" })),
    );

    const defaultClient = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const count = await defaultClient.contractCount();
    expect(count).toBe(3);
    expect(server.getAccount).not.toHaveBeenCalled();
    expect(server.sendTransaction).not.toHaveBeenCalled();
    expect(server.simulateTransaction).toHaveBeenCalledTimes(1);

    const tx = server.simulateTransaction.mock.calls[0][0];
    expect(tx.source).toBe(DEFAULT_SIMULATION_SOURCE_ACCOUNT);

    const customClient = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      sourceAccount: CUSTOM_SOURCE_ACCOUNT,
      server,
    });
    await customClient.contractCount();
    const customTx = server.simulateTransaction.mock.calls[1][0];
    expect(customTx.source).toBe(CUSTOM_SOURCE_ACCOUNT);
  });

  it("decodes getContract and returns null when not registered", async () => {
    const server = makeMockServer();
    server.simulateTransaction
      .mockResolvedValueOnce(
        okSimulation(
          makeRegisteredContractScVal(
            ESCROW_CONTRACT_ID,
            ContractKind.BountyEscrow,
            2,
          ),
        ),
      )
      .mockResolvedValueOnce(okSimulation(xdr.ScVal.scvVoid()));

    const client = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const found = await client.getContract(ESCROW_CONTRACT_ID);
    expect(found).toEqual({
      address: ESCROW_CONTRACT_ID,
      kind: ContractKind.BountyEscrow,
      version: 2,
    });

    const missing = await client.getContract(PROGRAM_CONTRACT_ID);
    expect(missing).toBeNull();
  });

  it("throws NotFoundError from getContractOrThrow when entry is absent", async () => {
    const server = makeMockServer();
    server.simulateTransaction.mockResolvedValue(
      okSimulation(xdr.ScVal.scvVoid()),
    );

    const client = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    await expect(
      client.getContractOrThrow(PROGRAM_CONTRACT_ID),
    ).rejects.toBeInstanceOf(NotFoundError);
  });

  it("paginates registered contracts via fetchAllContracts and iterateContracts", async () => {
    const server = makeMockServer();
    const page0 = xdr.ScVal.scvVec([
      makeRegisteredContractScVal(
        ESCROW_CONTRACT_ID,
        ContractKind.BountyEscrow,
        1,
      ),
      makeRegisteredContractScVal(
        PROGRAM_CONTRACT_ID,
        ContractKind.ProgramEscrow,
        2,
      ),
    ]);
    const page1 = xdr.ScVal.scvVec([
      makeRegisteredContractScVal(
        TOKEN_CONTRACT_ID,
        ContractKind.GrainlifyCore,
        3,
      ),
    ]);

    server.simulateTransaction
      .mockResolvedValueOnce(okSimulation(page0))
      .mockResolvedValueOnce(okSimulation(page1))
      .mockResolvedValueOnce(okSimulation(page0))
      .mockResolvedValueOnce(okSimulation(page1));

    const client = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const all = await client.fetchAllContracts(2);
    expect(all).toHaveLength(3);
    expect(all.map((c) => c.kind)).toEqual([
      ContractKind.BountyEscrow,
      ContractKind.ProgramEscrow,
      ContractKind.GrainlifyCore,
    ]);

    const iterated = [];
    for await (const entry of client.iterateContracts(2)) {
      iterated.push(entry);
    }
    expect(iterated).toEqual(all);
  });

  it("handles empty contract registry gracefully", async () => {
    const server = makeMockServer();
    server.simulateTransaction.mockResolvedValue(
      okSimulation(xdr.ScVal.scvVec([])),
    );

    const client = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    expect(await client.fetchAllContracts(10)).toEqual([]);
    expect(server.simulateTransaction).toHaveBeenCalledTimes(1);
  });

  it("decodes queryProgramDataCached, queryFeeConfigCached, and queryProgramBalanceAndFee with bigint i128 values", async () => {
    const server = makeMockServer();
    server.simulateTransaction
      .mockResolvedValueOnce(okSimulation(makeProgramDataScVal("grant-2026")))
      .mockResolvedValueOnce(okSimulation(makeFeeConfigScVal()))
      .mockResolvedValueOnce(
        okSimulation(
          xdr.ScVal.scvVec([
            makeProgramDataScVal("grant-2026"),
            makeFeeConfigScVal(),
          ]),
        ),
      );

    const client = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const program = await client.queryProgramDataCached(
      PROGRAM_CONTRACT_ID,
      "grant-2026",
    );
    expect(program.program_id).toBe("grant-2026");
    expect(program.total_funds).toBe(10_000_000_000n);
    expect(program.remaining_balance).toBe(7_500_000_000n);
    expect(program.initial_liquidity).toBe(1_000_000_000n);
    expect(program.payout_history).toHaveLength(1);
    expect(program.payout_history[0].amount).toBe(2_500_000_000n);

    const fees = await client.queryFeeConfigCached(PROGRAM_CONTRACT_ID);
    expect(fees.lock_fee_rate).toBe(100n);
    expect(fees.payout_fee_rate).toBe(50n);
    expect(fees.lock_fixed_fee).toBe(1_000n);
    expect(fees.payout_fixed_fee).toBe(500n);
    expect(fees.fee_enabled).toBe(true);
    expect(fees.insurance_reserve_bps).toBe(250);

    const [aggProgram, aggFees] = await client.queryProgramBalanceAndFee(
      PROGRAM_CONTRACT_ID,
      "grant-2026",
    );
    expect(aggProgram.remaining_balance).toBe(7_500_000_000n);
    expect(aggFees.payout_fee_rate).toBe(50n);
  });

  it("distinguishes NotFoundError from NetworkError (RPC failure) and ContractError", async () => {
    const server = makeMockServer();
    const client = new ViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    // 1. Not found simulation error
    server.simulateTransaction.mockResolvedValueOnce(
      errorSimulation("HostError: Error(Contract, #202) Bounty not found"),
    );
    await expect(
      client.queryProgramDataCached(PROGRAM_CONTRACT_ID, "missing-prog"),
    ).rejects.toBeInstanceOf(NotFoundError);

    // 2. RPC / network connection failure
    const connErr = Object.assign(new Error("connect ECONNREFUSED"), {
      code: "ECONNREFUSED",
    });
    server.simulateTransaction.mockRejectedValueOnce(connErr);
    await expect(
      client.queryProgramDataCached(PROGRAM_CONTRACT_ID, "prog-1"),
    ).rejects.toBeInstanceOf(NetworkError);

    // 3. HTTP 503 from RPC
    const httpErr = Object.assign(new Error("Service Unavailable"), {
      response: { status: 503 },
    });
    server.simulateTransaction.mockRejectedValueOnce(httpErr);
    await expect(client.contractCount()).rejects.toMatchObject({
      name: "NetworkError",
      statusCode: 503,
    });

    // 4. Other contract error (e.g., #4 InvalidPagination)
    server.simulateTransaction.mockResolvedValueOnce(
      errorSimulation("HostError: Error(Contract, #4)"),
    );
    await expect(client.listContracts(999, 10)).rejects.toBeInstanceOf(
      ContractError,
    );
  });
});

// ---------------------------------------------------------------------------
// EscrowViewFacadeClient
// ---------------------------------------------------------------------------

describe("EscrowViewFacadeClient", () => {
  it("decodes getEscrowSummary with bigint amounts and returns null when missing", async () => {
    const server = makeMockServer();
    server.simulateTransaction
      .mockResolvedValueOnce(
        okSimulation(
          makeEscrowSummaryScVal({
            bounty_id: 7n,
            amount: 123_456_789_012_345n,
            remaining_amount: 100_000_000_000_000n,
            status: EscrowStatus.PartiallyRefunded,
            is_paused: true,
          }),
        ),
      )
      .mockResolvedValueOnce(okSimulation(xdr.ScVal.scvVoid()));

    const client = new EscrowViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const summary = await client.getEscrowSummary(ESCROW_CONTRACT_ID, 7n);
    expect(summary).toEqual({
      bounty_id: 7n,
      depositor: DEPOSITOR_ADDRESS,
      amount: 123_456_789_012_345n,
      remaining_amount: 100_000_000_000_000n,
      status: EscrowStatus.PartiallyRefunded,
      deadline: 1_800_000_000n,
      repo_id: 101n,
      issue_id: 202n,
      bounty_type: "feature",
      is_paused: true,
    });
    expect(typeof summary?.amount).toBe("bigint");
    expect(typeof summary?.remaining_amount).toBe("bigint");

    const missing = await client.getEscrowSummary(ESCROW_CONTRACT_ID, 999n);
    expect(missing).toBeNull();
  });

  it("throws NotFoundError from getEscrowSummaryOrThrow when missing", async () => {
    const server = makeMockServer();
    server.simulateTransaction.mockResolvedValue(
      okSimulation(xdr.ScVal.scvVoid()),
    );

    const client = new EscrowViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    await expect(
      client.getEscrowSummaryOrThrow(ESCROW_CONTRACT_ID, 999n),
    ).rejects.toBeInstanceOf(NotFoundError);
  });

  it("decodes UserPortfolio including empty portfolios", async () => {
    const server = makeMockServer();
    const populatedPortfolio = scMap([
      [
        "as_depositor",
        xdr.ScVal.scvVec([
          makeEscrowSummaryScVal({ bounty_id: 1n, status: EscrowStatus.Locked }),
        ]),
      ],
      [
        "as_beneficiary",
        xdr.ScVal.scvVec([
          makeEscrowSummaryScVal({
            bounty_id: 2n,
            status: EscrowStatus.Released,
            remaining_amount: 0n,
          }),
        ]),
      ],
    ]);
    const emptyPortfolio = scMap([
      ["as_depositor", xdr.ScVal.scvVec([])],
      ["as_beneficiary", xdr.ScVal.scvVec([])],
    ]);

    server.simulateTransaction
      .mockResolvedValueOnce(okSimulation(populatedPortfolio))
      .mockResolvedValueOnce(okSimulation(emptyPortfolio));

    const client = new EscrowViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const portfolio = await client.getUserPortfolio(
      ESCROW_CONTRACT_ID,
      USER_ADDRESS,
    );
    expect(portfolio.as_depositor).toHaveLength(1);
    expect(portfolio.as_depositor[0].bounty_id).toBe(1n);
    expect(portfolio.as_depositor[0].status).toBe(EscrowStatus.Locked);
    expect(portfolio.as_beneficiary).toHaveLength(1);
    expect(portfolio.as_beneficiary[0].bounty_id).toBe(2n);
    expect(portfolio.as_beneficiary[0].remaining_amount).toBe(0n);

    const empty = await client.getUserPortfolio(
      ESCROW_CONTRACT_ID,
      DEPOSITOR_ADDRESS,
    );
    expect(empty).toEqual({ as_depositor: [], as_beneficiary: [] });
  });

  it("batches escrow summaries with fetchAllEscrowSummaries and iterateEscrowSummaries", async () => {
    const server = makeMockServer();
    const batch0 = xdr.ScVal.scvVec([
      makeEscrowSummaryScVal({ bounty_id: 1n }),
      makeEscrowSummaryScVal({ bounty_id: 2n }),
    ]);
    const batch1 = xdr.ScVal.scvVec([
      makeEscrowSummaryScVal({ bounty_id: 3n }),
    ]);

    server.simulateTransaction
      .mockResolvedValueOnce(okSimulation(batch0))
      .mockResolvedValueOnce(okSimulation(batch1))
      .mockResolvedValueOnce(okSimulation(batch0))
      .mockResolvedValueOnce(okSimulation(batch1));

    const client = new EscrowViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const ids = [1n, 2n, 3n];
    const all = await client.fetchAllEscrowSummaries(ESCROW_CONTRACT_ID, ids, 2);
    expect(all.map((s) => s.bounty_id)).toEqual([1n, 2n, 3n]);

    const iterated = [];
    for await (const s of client.iterateEscrowSummaries(
      ESCROW_CONTRACT_ID,
      ids,
      2,
    )) {
      iterated.push(s.bounty_id);
    }
    expect(iterated).toEqual([1n, 2n, 3n]);

    // Empty input does not call RPC
    expect(
      await client.fetchAllEscrowSummaries(ESCROW_CONTRACT_ID, [], 2),
    ).toEqual([]);
    expect(server.simulateTransaction).toHaveBeenCalledTimes(4);
  });

  it("decodes queryAllDelegates and handles empty results", async () => {
    const server = makeMockServer();
    const delegatesVec = xdr.ScVal.scvVec([
      scMap([
        ["program_id", nativeToScVal("prog-1", { type: "string" })],
        ["delegate", Address.fromString(USER_ADDRESS).toScVal()],
        ["permissions", nativeToScVal(3, { type: "u32" })],
      ]),
      scMap([
        ["program_id", nativeToScVal("prog-2", { type: "string" })],
        ["delegate", xdr.ScVal.scvVoid()],
        ["permissions", nativeToScVal(0, { type: "u32" })],
      ]),
    ]);

    server.simulateTransaction
      .mockResolvedValueOnce(okSimulation(delegatesVec))
      .mockResolvedValueOnce(okSimulation(xdr.ScVal.scvVec([])));

    const client = new EscrowViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    const delegates = await client.queryAllDelegates(
      PROGRAM_CONTRACT_ID,
      "prog-1",
    );
    expect(delegates).toEqual([
      { program_id: "prog-1", delegate: USER_ADDRESS, permissions: 3 },
      { program_id: "prog-2", delegate: null, permissions: 0 },
    ]);

    const empty = await client.queryAllDelegates(
      PROGRAM_CONTRACT_ID,
      "prog-none",
    );
    expect(empty).toEqual([]);
  });

  it("validates inputs before calling RPC", async () => {
    const server = makeMockServer();
    const client = new EscrowViewFacadeClient({
      contractId: FACADE_CONTRACT_ID,
      rpcUrl: RPC_URL,
      networkPassphrase: NETWORK_PASSPHRASE,
      server,
    });

    await expect(
      client.getEscrowSummary("INVALID", 1n),
    ).rejects.toBeInstanceOf(ValidationError);
    await expect(
      client.getEscrowSummary(ESCROW_CONTRACT_ID, -1n),
    ).rejects.toBeInstanceOf(ValidationError);
    await expect(
      client.getUserPortfolio(ESCROW_CONTRACT_ID, "NOT_AN_ADDRESS"),
    ).rejects.toBeInstanceOf(ValidationError);
    expect(server.simulateTransaction).not.toHaveBeenCalled();
  });
});
