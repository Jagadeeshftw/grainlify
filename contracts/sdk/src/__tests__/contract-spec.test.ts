import {
  Address,
  Keypair,
  nativeToScVal,
  xdr,
} from "@stellar/stellar-sdk";
import {
  ContractSpec,
  PROGRAM_ESCROW_SPEC,
  SEP41_TOKEN_SPEC,
} from "../contract-spec";
import { ProgramEscrowClient } from "../program-escrow-client";
import { ValidationError } from "../errors";

const SIGNER = Keypair.random();
const RECIPIENT_1 = Keypair.random().publicKey();
const RECIPIENT_2 = Keypair.random().publicKey();
const ADMIN = Keypair.random().publicKey();
const TOKEN_CONTRACT_ID =
  "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4";
const ESCROW_CONTRACT_ID =
  "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM";

function toXdrBase64(vals: xdr.ScVal[]): string[] {
  return vals.map((v) => v.toXDR("base64"));
}

/**
 * Reference implementation of the legacy hand-written switch encoder
 * (removed from `ProgramEscrowClient`) for golden-equivalence assertions.
 */
function legacyEncodeArguments(
  method: string,
  args: any[],
  signer: Keypair,
): xdr.ScVal[] {
  const address = (value: string) => Address.fromString(value).toScVal();
  const i128 = (value: bigint) => nativeToScVal(value, { type: "i128" });
  const str = (value: string) => nativeToScVal(value, { type: "string" });
  const vector = (values: xdr.ScVal[]) => xdr.ScVal.scvVec(values);

  switch (method) {
    case "init_program":
      return [
        str(args[0]),
        address(args[1]),
        address(args[2]),
        address(signer.publicKey()),
        args[3] > 0n ? i128(args[3]) : xdr.ScVal.scvVoid(),
        xdr.ScVal.scvVoid(),
      ];
    case "lock_program_funds":
      return [i128(args[0])];
    case "batch_payout":
      return [
        vector(args[0].map((value: string) => address(value))),
        vector(args[1].map((value: bigint) => i128(value))),
      ];
    case "single_payout":
      return [address(args[0]), i128(args[1])];
    case "publish_program":
      return [str(args[0]), address(args[1])];
    case "transfer":
      return [address(args[0]), address(args[1]), i128(args[2])];
    case "get_program_info":
    case "get_remaining_balance":
      return [];
    case "trigger_program_releases":
      return [xdr.ScVal.scvVoid()];
    case "create_program_release_schedule":
      return [
        address(args[0]),
        i128(args[1]),
        nativeToScVal(BigInt(args[2]), { type: "u64" }),
      ];
    default:
      throw new Error(`No legacy encoder for ${method}`);
  }
}

describe("Golden-equivalence: spec-driven encoding vs legacy switch encoder", () => {
  it("init_program (with positive initial_liquidity)", () => {
    const legacy = legacyEncodeArguments(
      "init_program",
      ["prog-1", RECIPIENT_1, TOKEN_CONTRACT_ID, 500_000n],
      SIGNER,
    );
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("init_program", {
      program_id: "prog-1",
      authorized_payout_key: RECIPIENT_1,
      token_address: TOKEN_CONTRACT_ID,
      creator: SIGNER.publicKey(),
      initial_liquidity: 500_000n,
      reference_hash: null,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("init_program (with zero initial_liquidity -> Option::None)", () => {
    const legacy = legacyEncodeArguments(
      "init_program",
      ["prog-2", RECIPIENT_1, TOKEN_CONTRACT_ID, 0n],
      SIGNER,
    );
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("init_program", {
      program_id: "prog-2",
      authorized_payout_key: RECIPIENT_1,
      token_address: TOKEN_CONTRACT_ID,
      creator: SIGNER.publicKey(),
      initial_liquidity: null,
      reference_hash: null,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("lock_program_funds", () => {
    const legacy = legacyEncodeArguments("lock_program_funds", [10_000_000n], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("lock_program_funds", {
      amount: 10_000_000n,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("batch_payout", () => {
    const recipients = [RECIPIENT_1, RECIPIENT_2];
    const amounts = [1_000n, 2_500n];
    const legacy = legacyEncodeArguments("batch_payout", [recipients, amounts], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("batch_payout", {
      recipients,
      amounts,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("single_payout", () => {
    const legacy = legacyEncodeArguments("single_payout", [RECIPIENT_1, 42_000n], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("single_payout", {
      recipient: RECIPIENT_1,
      amount: 42_000n,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("publish_program", () => {
    const legacy = legacyEncodeArguments("publish_program", ["prog-1", ADMIN], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("publish_program", {
      program_id: "prog-1",
      caller: ADMIN,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("transfer (SEP-41 token spec)", () => {
    const legacy = legacyEncodeArguments(
      "transfer",
      [SIGNER.publicKey(), ESCROW_CONTRACT_ID, 99_000n],
      SIGNER,
    );
    const specEncoded = SEP41_TOKEN_SPEC.encodeArgs("transfer", {
      from: SIGNER.publicKey(),
      to: ESCROW_CONTRACT_ID,
      amount: 99_000n,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("get_program_info", () => {
    const legacy = legacyEncodeArguments("get_program_info", [], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("get_program_info", {});
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("get_remaining_balance", () => {
    const legacy = legacyEncodeArguments("get_remaining_balance", [], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("get_remaining_balance", {});
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("trigger_program_releases", () => {
    const legacy = legacyEncodeArguments("trigger_program_releases", [null], SIGNER);
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs("trigger_program_releases", {
      epoch_id: null,
    });
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });

  it("create_program_release_schedule", () => {
    const ts = 1_750_000_000;
    const legacy = legacyEncodeArguments(
      "create_program_release_schedule",
      [RECIPIENT_1, 7_500n, ts],
      SIGNER,
    );
    const specEncoded = PROGRAM_ESCROW_SPEC.encodeArgs(
      "create_program_release_schedule",
      {
        recipient: RECIPIENT_1,
        amount: 7_500n,
        release_timestamp: ts,
      },
    );
    expect(toXdrBase64(specEncoded)).toEqual(toXdrBase64(legacy));
  });
});

describe("ContractSpec validation errors", () => {
  it("throws ValidationError with allowed methods for unknown method", () => {
    expect(() =>
      PROGRAM_ESCROW_SPEC.encodeArgs("non_existent_method", {}),
    ).toThrow(ValidationError);

    try {
      PROGRAM_ESCROW_SPEC.encodeArgs("non_existent_method", {});
    } catch (err: any) {
      expect(err).toBeInstanceOf(ValidationError);
      expect(err.field).toBe("method");
      expect(err.message).toContain("non_existent_method");
      expect(err.message).toContain("init_program");
      expect(err.message).toContain("lock_program_funds");
    }
  });

  it("throws ValidationError with allowed argument names for unknown arg", () => {
    try {
      PROGRAM_ESCROW_SPEC.encodeArgs("single_payout", {
        recipient: RECIPIENT_1,
        amount: 100n,
        bogus_arg: "nope",
      });
      fail("Expected ValidationError");
    } catch (err: any) {
      expect(err).toBeInstanceOf(ValidationError);
      expect(err.field).toBe("bogus_arg");
      expect(err.message).toContain("bogus_arg");
      expect(err.message).toContain("recipient");
      expect(err.message).toContain("amount");
    }
  });

  it("throws ValidationError with allowed argument names for missing required arg", () => {
    try {
      PROGRAM_ESCROW_SPEC.encodeArgs("single_payout", {
        recipient: RECIPIENT_1,
      });
      fail("Expected ValidationError");
    } catch (err: any) {
      expect(err).toBeInstanceOf(ValidationError);
      expect(err.field).toBe("amount");
      expect(err.message).toContain("amount");
      expect(err.message).toContain("recipient");
    }
  });

  it("throws ValidationError for unknown struct field", () => {
    expect(() =>
      PROGRAM_ESCROW_SPEC.encodeArgs("query_payout_history", {
        filter: { unknown_field: 123 },
        offset: 0,
        limit: 10,
      }),
    ).toThrow(ValidationError);
  });

  it("throws ValidationError for invalid enum variant", () => {
    expect(() =>
      PROGRAM_ESCROW_SPEC.encodeArgs("set_program_status", {
        program_id: "prog-1",
        status: "NotAStatus",
      }),
    ).toThrow(ValidationError);
  });
});

describe("Additional entrypoints (10+ beyond legacy switch, incl. structs, Option, Vec, enums)", () => {
  it("1. query_payout_history (struct PayoutQueryFilter with Option fields + u32)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("query_payout_history", {
      filter: {
        recipient: RECIPIENT_1,
        min_amount: 100n,
        max_timestamp: 1_800_000_000,
      },
      offset: 10,
      limit: 25,
    });
    expect(encoded).toHaveLength(3);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvMap());
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvU32());
    expect(encoded[2].switch()).toBe(xdr.ScValType.scvU32());
  });

  it("2. query_release_schedules (struct ScheduleQueryFilter with Option<bool>)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("query_release_schedules", {
      filter: {
        recipient: RECIPIENT_2,
        released: false,
        min_release_timestamp: 1_700_000_000,
      },
      offset: 0,
      limit: 50,
    });
    expect(encoded).toHaveLength(3);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvMap());
  });

  it("3. set_paused (all Option<bool> and Option<String>)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("set_paused", {
      lock: true,
      release: false,
      reason: "maintenance",
    });
    expect(encoded).toHaveLength(4);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvBool());
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvBool());
    expect(encoded[2].switch()).toBe(xdr.ScValType.scvVoid()); // refund omitted -> None
    expect(encoded[3].switch()).toBe(xdr.ScValType.scvString());
  });

  it("4. update_fee_config (7 Option<...> args)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("update_fee_config", {
      lock_fee_rate: 50n,
      fee_recipient: ADMIN,
      fee_enabled: true,
      insurance_reserve_bps: 250,
    });
    expect(encoded).toHaveLength(7);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvI128());
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvVoid());
    expect(encoded[4].switch()).toBe(xdr.ScValType.scvAddress());
    expect(encoded[5].switch()).toBe(xdr.ScValType.scvBool());
    expect(encoded[6].switch()).toBe(xdr.ScValType.scvU32());
  });

  it("5. configure_circuit_breaker (Address + 3x u32)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("configure_circuit_breaker", {
      caller: ADMIN,
      failure_threshold: 5,
      success_threshold: 3,
      max_error_log: 20,
    });
    expect(encoded).toHaveLength(4);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvAddress());
    expect(encoded[1].u32()).toBe(5);
    expect(encoded[2].u32()).toBe(3);
    expect(encoded[3].u32()).toBe(20);
  });

  it("6. set_circuit_admin (Address + Option<Address>)", () => {
    const withCaller = PROGRAM_ESCROW_SPEC.encodeArgs("set_circuit_admin", {
      new_admin: ADMIN,
      caller: RECIPIENT_1,
    });
    expect(withCaller[1].switch()).toBe(xdr.ScValType.scvAddress());

    const withoutCaller = PROGRAM_ESCROW_SPEC.encodeArgs("set_circuit_admin", {
      new_admin: ADMIN,
    });
    expect(withoutCaller[1].switch()).toBe(xdr.ScValType.scvVoid());
  });

  it("7. propose_controller (String, Address, Address)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("propose_controller", {
      program_id: "hack-2026",
      caller: ADMIN,
      proposed_controller: RECIPIENT_2,
    });
    expect(encoded).toHaveLength(3);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvString());
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvAddress());
    expect(encoded[2].switch()).toBe(xdr.ScValType.scvAddress());
  });

  it("8. set_program_spend_threshold (String, i128)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("set_program_spend_threshold", {
      program_id: "hack-2026",
      threshold_amount: 1_000_000n,
    });
    expect(encoded).toHaveLength(2);
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvI128());
  });

  it("9. withdraw_insurance_reserve (Address, i128)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("withdraw_insurance_reserve", {
      target: ADMIN,
      amount: 25_000n,
    });
    expect(encoded).toHaveLength(2);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvAddress());
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvI128());
  });

  it("10. set_program_status (enum ProgramStatus)", () => {
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("set_program_status", {
      program_id: "hack-2026",
      status: "Active",
    });
    expect(encoded).toHaveLength(2);
    expect(encoded[1].switch()).toBe(xdr.ScValType.scvVec());
    const vec = encoded[1].vec()!;
    expect(vec).toHaveLength(1);
    expect(vec[0].sym().toString()).toBe("Active");
  });

  it("11. get_program_info_v2 (read-only with String)", () => {
    expect(PROGRAM_ESCROW_SPEC.isReadOnly("get_program_info_v2")).toBe(true);
    const encoded = PROGRAM_ESCROW_SPEC.encodeArgs("get_program_info_v2", {
      program_id: "hack-2026",
    });
    expect(encoded).toHaveLength(1);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvString());
  });

  it("12. get_circuit_breaker_status, get_fee_config, get_insurance_reserve_balance", () => {
    for (const method of [
      "get_circuit_breaker_status",
      "get_cb_schema_version",
      "get_circuit_error_log",
      "get_fee_config",
      "get_insurance_reserve_balance",
    ]) {
      expect(PROGRAM_ESCROW_SPEC.isReadOnly(method)).toBe(true);
      expect(PROGRAM_ESCROW_SPEC.encodeArgs(method, {})).toEqual([]);
    }
  });

  it("supports custom contract specs with Vec, Map, tagged enums, and u32Enum", () => {
    const customSpec = new ContractSpec({
      contractName: "CustomContract",
      types: {
        Action: {
          kind: "enum",
          variants: [
            { name: "Noop" },
            { name: "Transfer", fields: ["Address", "i128"] },
          ],
        },
        Tier: {
          kind: "u32Enum",
          variants: { Bronze: 1, Silver: 2, Gold: 3 },
        },
      },
      entrypoints: [
        {
          name: "execute_batch",
          readOnly: false,
          parameters: [
            { name: "actions", type: "Vec<Action>" },
            { name: "tier", type: "Tier" },
            { name: "meta", type: "Map<String, u32>" },
          ],
        },
      ],
    });

    const encoded = customSpec.encodeArgs("execute_batch", {
      actions: [
        "Noop",
        { tag: "Transfer", values: [RECIPIENT_1, 500n] },
      ],
      tier: "Gold",
      meta: { priority: 1 },
    });

    expect(encoded).toHaveLength(3);
    expect(encoded[0].switch()).toBe(xdr.ScValType.scvVec());
    expect(encoded[1].u32()).toBe(3);
    expect(encoded[2].switch()).toBe(xdr.ScValType.scvMap());
  });
});

describe("Read-only derivation from manifest & ProgramEscrowClient.invoke/simulate", () => {
  it("ContractSpec.fromManifest derives read-only methods from view and no-auth getters", () => {
    const spec = ContractSpec.fromManifest({
      contract_name: "DemoContract",
      entrypoints: {
        view: [{ name: "liveness_watchdog", parameters: [] }],
        public: [
          { name: "get_balance", authorization: "none", parameters: [] },
          { name: "deposit", authorization: "caller", parameters: [{ name: "amount", type: "i128" }] },
        ],
        admin: [{ name: "pause", authorization: "admin", parameters: [] }],
      },
    });

    expect(spec.isReadOnly("liveness_watchdog")).toBe(true);
    expect(spec.isReadOnly("get_balance")).toBe(true);
    expect(spec.isReadOnly("deposit")).toBe(false);
    expect(spec.isReadOnly("pause")).toBe(false);
    expect(Array.from(spec.getReadOnlyMethods()).sort()).toEqual([
      "get_balance",
      "liveness_watchdog",
    ]);
  });

  it("client.invoke and client.simulate reject unknown methods with ValidationError", async () => {
    const client = new ProgramEscrowClient({
      contractId: ESCROW_CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
      sourceKeypair: SIGNER,
    });

    await expect(client.invoke("does_not_exist", {})).rejects.toThrow(
      ValidationError,
    );
    await expect(client.simulate("does_not_exist", {})).rejects.toThrow(
      ValidationError,
    );
  });

  it("client.simulate does not submit transactions even for write entrypoints", async () => {
    const client = new ProgramEscrowClient({
      contractId: ESCROW_CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
      sourceKeypair: SIGNER,
    });

    const mockServer = {
      getAccount: jest.fn().mockResolvedValue({
        accountId: () => SIGNER.publicKey(),
        sequenceNumber: () => "1",
        incrementSequenceNumber: () => {},
      }),
      simulateTransaction: jest.fn().mockResolvedValue({
        id: "sim-1",
        result: { retval: nativeToScVal(42n, { type: "i128" }) },
      }),
      sendTransaction: jest.fn(),
    };
    (client as any).server = mockServer;

    const result = await client.simulate<bigint>("lock_program_funds", {
      amount: 1_000n,
    });

    expect(result).toBe(42n);
    expect(mockServer.simulateTransaction).toHaveBeenCalledTimes(1);
    expect(mockServer.sendTransaction).not.toHaveBeenCalled();
  });

  it("client.invoke simulates read-only entrypoints from spec without submitting", async () => {
    const client = new ProgramEscrowClient({
      contractId: ESCROW_CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
      sourceKeypair: SIGNER,
    });

    const mockServer = {
      getAccount: jest.fn().mockResolvedValue({
        accountId: () => SIGNER.publicKey(),
        sequenceNumber: () => "1",
        incrementSequenceNumber: () => {},
      }),
      simulateTransaction: jest.fn().mockResolvedValue({
        id: "sim-2",
        result: { retval: nativeToScVal(1, { type: "u32" }) },
      }),
      sendTransaction: jest.fn(),
    };
    (client as any).server = mockServer;

    const version = await client.invoke<number>("get_cb_schema_version", {});
    expect(version).toBe(1);
    expect(mockServer.simulateTransaction).toHaveBeenCalledTimes(1);
    expect(mockServer.sendTransaction).not.toHaveBeenCalled();
  });
});
