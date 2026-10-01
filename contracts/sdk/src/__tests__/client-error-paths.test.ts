/**
 * Tests that contract errors, network failures, and edge-case inputs
 * surface as the correct typed SDK errors when flowing through
 * ProgramEscrowClient method calls.
 *
 * Unlike the unit-level error-handling / error-mapping tests, these
 * exercises simulate realistic failure scenarios by injecting errors
 * at the `invokeContract` boundary and asserting on what the public
 * client API exposes to callers.
 */
import { ProgramEscrowClient } from '../program-escrow-client';
import {
  ContractError,
  ContractErrorCode,
  NetworkError,
  ValidationError,
  SDKError,
} from '../errors';
import { Keypair } from '@stellar/stellar-sdk';
 
// ── Helpers ────────────────────────────────────────────────────────────
 
const VALID_ADDRESS = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
const VALID_ADDRESS_2 = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';
const TOKEN_CONTRACT_ID = 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
const CONTRACT_ID = 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC';
const RPC_URL = 'https://soroban-testnet.stellar.org';
const PASSPHRASE = 'Test SDF Network ; September 2015';
 
function makeClient(): ProgramEscrowClient {
  return new ProgramEscrowClient({
    contractId: CONTRACT_ID,
    rpcUrl: RPC_URL,
    networkPassphrase: PASSPHRASE,
  });
}
 
/** Replace `invokeContract` so it throws the supplied error. */
function stubInvoke(client: ProgramEscrowClient, error: Error): void {
  (client as any).invokeContract = async () => {
    throw error;
  };
}
 
/** Replace `invokeContract` with a function that records calls then throws. */
function stubInvokeWithSpy(
  client: ProgramEscrowClient,
  error: Error,
): jest.Mock {
  const spy = jest.fn(async () => {
    throw error;
  });
  (client as any).invokeContract = spy;
  return spy;
}
 
// =======================================================================
// 1. Contract errors surfacing through every client method
// =======================================================================
describe('Contract errors through ProgramEscrowClient methods', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  // ── NOT_INITIALIZED ─────────────────────────────────────────────────
 
  describe('NOT_INITIALIZED', () => {
    beforeEach(() => {
      stubInvoke(client, new Error('Program not initialized'));
    });
 
    it('surfaces through getProgramInfo', async () => {
      try {
        await client.getProgramInfo();
        fail('Expected ContractError');
      } catch (err: any) {
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).toBe(ContractErrorCode.NOT_INITIALIZED);
      }
    });
 
    it('surfaces through getRemainingBalance', async () => {
      await expect(client.getRemainingBalance()).rejects.toThrow(ContractError);
      await expect(client.getRemainingBalance()).rejects.toMatchObject({
        code: ContractErrorCode.NOT_INITIALIZED,
      });
    });
 
    it('surfaces through singlePayout', async () => {
      await expect(
        client.singlePayout(VALID_ADDRESS, 1000n, keypair),
      ).rejects.toThrow(ContractError);
    });
 
    it('surfaces through batchPayout', async () => {
      await expect(
        client.batchPayout([VALID_ADDRESS], [500n], keypair),
      ).rejects.toThrow(ContractError);
    });
 
    it('surfaces through lockProgramFunds', async () => {
      await expect(
        client.lockProgramFunds(1000n, keypair),
      ).rejects.toThrow(ContractError);
    });
 
    it('surfaces through triggerProgramReleases', async () => {
      await expect(
        client.triggerProgramReleases(keypair),
      ).rejects.toThrow(ContractError);
    });
 
    it('surfaces through createProgramReleaseSchedule', async () => {
      await expect(
        client.createProgramReleaseSchedule(
          VALID_ADDRESS,
          1000n,
          Date.now() + 86400,
          keypair,
        ),
      ).rejects.toThrow(ContractError);
    });
  });
 
  // ── UNAUTHORIZED ────────────────────────────────────────────────────
 
  describe('UNAUTHORIZED', () => {
    it('maps require_auth failure to UNAUTHORIZED', async () => {
      stubInvoke(client, new Error('require_auth failed for account'));
      try {
        await client.singlePayout(VALID_ADDRESS, 500n, keypair);
        fail('Expected ContractError');
      } catch (err: any) {
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).toBe(ContractErrorCode.UNAUTHORIZED);
        expect(err.message).toContain('Unauthorized');
      }
    });
 
    it('maps "Unauthorized" string to UNAUTHORIZED', async () => {
      stubInvoke(client, new Error('Unauthorized access attempt'));
      await expect(
        client.batchPayout([VALID_ADDRESS], [100n], keypair),
      ).rejects.toMatchObject({ code: ContractErrorCode.UNAUTHORIZED });
    });
  });
 
  // ── INSUFFICIENT_BALANCE ────────────────────────────────────────────
 
  describe('INSUFFICIENT_BALANCE', () => {
    it('surfaces through singlePayout', async () => {
      stubInvoke(client, new Error('Insufficient balance'));
      try {
        await client.singlePayout(VALID_ADDRESS, 999999n, keypair);
        fail('Expected ContractError');
      } catch (err: any) {
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).toBe(ContractErrorCode.INSUFFICIENT_BALANCE);
      }
    });
 
    it('surfaces through batchPayout', async () => {
      stubInvoke(client, new Error('Insufficient balance for payout'));
      try {
        await client.batchPayout(
          [VALID_ADDRESS, VALID_ADDRESS_2],
          [1n, 1n],
          keypair,
        );
        fail('Expected ContractError');
      } catch (err: any) {
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).toBe(ContractErrorCode.INSUFFICIENT_BALANCE);
      }
    });
  });
 
  // ── ALREADY_INITIALIZED ─────────────────────────────────────────────
 
  describe('ALREADY_INITIALIZED', () => {
    it('surfaces through initProgram', async () => {
      stubInvoke(client, new Error('Program already initialized'));
      try {
        await client.initProgram(
          'my-program',
          VALID_ADDRESS,
          TOKEN_CONTRACT_ID,
          keypair,
        );
        fail('Expected ContractError');
      } catch (err: any) {
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).toBe(ContractErrorCode.ALREADY_INITIALIZED);
      }
    });
  });

  // ── OVERFLOW ────────────────────────────────────────────────────────

  describe('OVERFLOW', () => {
    it('surfaces through batchPayout with large amounts', async () => {
      stubInvoke(client, new Error('Payout amount overflow'));
      try {
        await client.batchPayout(
          [VALID_ADDRESS, VALID_ADDRESS_2],
          [1000n, 2000n],
          keypair,
        );
        fail('Expected ContractError');
      } catch (err: any) {
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).toBe(ContractErrorCode.OVERFLOW);
      }
    });
  });

  // ── EMPTY_BATCH / LENGTH_MISMATCH via contract (post-validation) ───

  describe('EMPTY_BATCH from contract layer', () => {
    it('maps contract-level empty batch error', async () => {
      // This simulates the contract itself rejecting an empty batch,
      // distinct from the client-side validation that catches it earlier.
      stubInvoke(client, new Error('Cannot process empty batch'));
      // Bypass client validation by calling handleError path directly
      const handled = (client as any).handleError(
        new Error('Cannot process empty batch'),
      );
      expect(handled).toBeInstanceOf(ContractError);
      expect(handled.code).toBe(ContractErrorCode.EMPTY_BATCH);
    });
  });
 
  describe('LENGTH_MISMATCH from contract layer', () => {
    it('maps contract-level length mismatch error', async () => {
      stubInvoke(
        client,
        new Error('Recipients and amounts must have the same length'),
      );
      const handled = (client as any).handleError(
        new Error('Recipients and amounts must have the same length'),
      );
      expect(handled).toBeInstanceOf(ContractError);
      expect(handled.code).toBe(ContractErrorCode.LENGTH_MISMATCH);
    });
  });
 
  // ── AMOUNT_BELOW_MIN / AMOUNT_ABOVE_MAX ─────────────────────────────
 
  describe('Amount policy errors', () => {
    it('maps AmountBelowMinimum through lockProgramFunds', async () => {
      stubInvoke(client, new Error('AmountBelowMinimum'));
      await expect(
        client.lockProgramFunds(1n, keypair),
      ).rejects.toMatchObject({ code: ContractErrorCode.AMOUNT_BELOW_MIN });
    });
 
    it('maps AmountAboveMaximum through lockProgramFunds', async () => {
      stubInvoke(client, new Error('AmountAboveMaximum'));
      await expect(
        client.lockProgramFunds(999999999n, keypair),
      ).rejects.toMatchObject({ code: ContractErrorCode.AMOUNT_ABOVE_MAX });
    });
  });
});
 
// =======================================================================
// 2. Network / transport errors through client methods
// =======================================================================
describe('Network errors through ProgramEscrowClient methods', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  it('ECONNREFUSED surfaces as NetworkError from getProgramInfo', async () => {
    const raw: any = new Error('connect ECONNREFUSED 127.0.0.1:9999');
    raw.code = 'ECONNREFUSED';
    stubInvoke(client, raw);
 
    try {
      await client.getProgramInfo();
      fail('Expected NetworkError');
    } catch (err: any) {
      expect(err).toBeInstanceOf(NetworkError);
      expect(err.code).toBe('NETWORK_ERROR');
      expect(err.cause).toBe(raw);
    }
  });
 
  it('ETIMEDOUT surfaces as NetworkError from singlePayout', async () => {
    const raw: any = new Error('request timed out');
    raw.code = 'ETIMEDOUT';
    stubInvoke(client, raw);
 
    await expect(
      client.singlePayout(VALID_ADDRESS, 100n, keypair),
    ).rejects.toBeInstanceOf(NetworkError);
  });
 
  it('ENOTFOUND surfaces as NetworkError from batchPayout', async () => {
    const raw: any = new Error('getaddrinfo ENOTFOUND');
    raw.code = 'ENOTFOUND';
    stubInvoke(client, raw);
 
    await expect(
      client.batchPayout([VALID_ADDRESS], [100n], keypair),
    ).rejects.toBeInstanceOf(NetworkError);
  });
 
  it('HTTP 429 Too Many Requests surfaces as NetworkError', async () => {
    const raw: any = new Error('Too Many Requests');
    raw.response = { status: 429 };
    stubInvoke(client, raw);
 
    try {
      await client.lockProgramFunds(500n, keypair);
      fail('Expected NetworkError');
    } catch (err: any) {
      expect(err).toBeInstanceOf(NetworkError);
      expect(err.statusCode).toBe(429);
    }
  });
 
  it('HTTP 502 Bad Gateway surfaces as NetworkError', async () => {
    const raw: any = new Error('Bad Gateway');
    raw.response = { status: 502 };
    stubInvoke(client, raw);
 
    try {
      await client.getRemainingBalance();
      fail('Expected NetworkError');
    } catch (err: any) {
      expect(err).toBeInstanceOf(NetworkError);
      expect(err.statusCode).toBe(502);
      expect(err.message).toContain('502');
    }
  });
 
  it('preserves RPC URL in NetworkError message', async () => {
    const customUrl = 'https://my-custom-rpc.example.org';
    const customClient = new ProgramEscrowClient({
      contractId: CONTRACT_ID,
      rpcUrl: customUrl,
      networkPassphrase: PASSPHRASE,
    });
    const raw: any = new Error('connection reset');
    raw.code = 'ECONNREFUSED';
    stubInvoke(customClient, raw);
 
    try {
      await customClient.getProgramInfo();
      fail('Expected NetworkError');
    } catch (err: any) {
      expect(err).toBeInstanceOf(NetworkError);
      expect(err.message).toContain(customUrl);
    }
  });
});
 
// =======================================================================
// 3. Error discrimination — callers can branch on instanceof
// =======================================================================
describe('Error type discrimination at the call site', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  it('ValidationError is distinguishable from ContractError', async () => {
    // Trigger a validation error (no mock needed)
    try {
      await client.singlePayout('bad-addr', 100n, keypair);
      fail('Expected an error');
    } catch (err: any) {
      expect(err).toBeInstanceOf(ValidationError);
      expect(err).not.toBeInstanceOf(ContractError);
      expect(err).not.toBeInstanceOf(NetworkError);
    }
  });
 
  it('ContractError is distinguishable from NetworkError', async () => {
    stubInvoke(client, new Error('Program not initialized'));
    try {
      await client.getProgramInfo();
      fail('Expected an error');
    } catch (err: any) {
      expect(err).toBeInstanceOf(ContractError);
      expect(err).not.toBeInstanceOf(NetworkError);
      expect(err).not.toBeInstanceOf(ValidationError);
    }
  });
 
  it('NetworkError is distinguishable from ContractError', async () => {
    const raw: any = new Error('timeout');
    raw.code = 'ETIMEDOUT';
    stubInvoke(client, raw);
    try {
      await client.getProgramInfo();
      fail('Expected an error');
    } catch (err: any) {
      expect(err).toBeInstanceOf(NetworkError);
      expect(err).not.toBeInstanceOf(ContractError);
      expect(err).not.toBeInstanceOf(ValidationError);
    }
  });
 
  it('all SDK errors share the SDKError base class', async () => {
    // Validation path
    try {
      await client.lockProgramFunds(0n, keypair);
    } catch (err: any) {
      expect(err).toBeInstanceOf(SDKError);
    }
 
    // Contract path
    stubInvoke(client, new Error('Insufficient balance'));
    try {
      await client.singlePayout(VALID_ADDRESS, 100n, keypair);
    } catch (err: any) {
      expect(err).toBeInstanceOf(SDKError);
    }
 
    // Network path
    const raw: any = new Error('fail');
    raw.code = 'ECONNREFUSED';
    stubInvoke(client, raw);
    try {
      await client.getProgramInfo();
    } catch (err: any) {
      expect(err).toBeInstanceOf(SDKError);
    }
  });
});
 
// =======================================================================
// 4. Error property preservation end-to-end
// =======================================================================
describe('Error properties preserved through client', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  it('ContractError retains name and code after handleError', async () => {
    stubInvoke(client, new Error('require_auth failed'));
    try {
      await client.singlePayout(VALID_ADDRESS, 100n, keypair);
      fail('Expected ContractError');
    } catch (err: any) {
      expect(err.name).toBe('ContractError');
      expect(err.code).toBe(ContractErrorCode.UNAUTHORIZED);
      expect(typeof err.message).toBe('string');
      expect(err.message.length).toBeGreaterThan(0);
    }
  });
 
  it('NetworkError retains statusCode and cause', async () => {
    const cause = new Error('upstream failure');
    (cause as any).response = { status: 503 };
    stubInvoke(client, cause);
 
    try {
      await client.getRemainingBalance();
      fail('Expected NetworkError');
    } catch (err: any) {
      expect(err.name).toBe('NetworkError');
      expect(err.statusCode).toBe(503);
      expect(err.cause).toBe(cause);
    }
  });
 
  it('ValidationError retains field name', async () => {
    try {
      await client.singlePayout('', 100n, keypair);
      fail('Expected ValidationError');
    } catch (err: any) {
      expect(err.name).toBe('ValidationError');
      expect(err.field).toBe('recipient');
    }
  });
 
  it('unknown contract error falls back to generic CONTRACT_ERROR', async () => {
    stubInvoke(client, new Error('something totally unexpected from chain'));
    try {
      await client.getProgramInfo();
      fail('Expected ContractError');
    } catch (err: any) {
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe('CONTRACT_ERROR');
      expect(err.message).toContain('something totally unexpected');
    }
  });
});
 
// =======================================================================
// 5. Already-typed errors pass through without double-wrapping
// =======================================================================
describe('Pre-typed errors are not double-wrapped', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  it('ContractError thrown by invokeContract is returned as-is', async () => {
    const original = new ContractError(
      'Program not initialized',
      ContractErrorCode.NOT_INITIALIZED,
      1,
    );
    stubInvoke(client, original);
 
    try {
      await client.getProgramInfo();
      fail('Expected ContractError');
    } catch (err: any) {
      expect(err).toBe(original);
      expect(err.contractErrorCode).toBe(1);
    }
  });
 
  it('NetworkError thrown by invokeContract is returned as-is', async () => {
    const original = new NetworkError('RPC down', 503, new Error('cause'));
    stubInvoke(client, original);
 
    try {
      await client.getRemainingBalance();
      fail('Expected NetworkError');
    } catch (err: any) {
      expect(err).toBe(original);
      expect(err.statusCode).toBe(503);
    }
  });
 
  it('ValidationError thrown by invokeContract is returned as-is', async () => {
    const original = new ValidationError('bad input', 'someField');
    stubInvoke(client, original);
 
    try {
      await client.getProgramInfo();
      fail('Expected ValidationError');
    } catch (err: any) {
      expect(err).toBe(original);
      expect(err.field).toBe('someField');
    }
  });
});
 
// =======================================================================
// 6. Retry semantics — client remains usable after errors
// =======================================================================
describe('Client recovery after errors', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  it('recovers from a contract error on subsequent call', async () => {
    let attempt = 0;
    (client as any).invokeContract = async () => {
      attempt++;
      if (attempt === 1) {
        throw new Error('Program not initialized');
      }
      return {
        program_id: 'p1',
        total_funds: 0n,
        remaining_balance: 0n,
        authorized_payout_key: VALID_ADDRESS,
        payout_history: [],
        token_address: VALID_ADDRESS_2,
      };
    };
 
    await expect(client.getProgramInfo()).rejects.toThrow(ContractError);
    const info = await client.getProgramInfo();
    expect(info.program_id).toBe('p1');
  });
 
  it('recovers from a network error on subsequent call', async () => {
    let attempt = 0;
    (client as any).invokeContract = async () => {
      attempt++;
      if (attempt === 1) {
        const err: any = new Error('timeout');
        err.code = 'ETIMEDOUT';
        throw err;
      }
      return 42000n;
    };
 
    await expect(client.getRemainingBalance()).rejects.toThrow(NetworkError);
    const balance = await client.getRemainingBalance();
    expect(balance).toBe(42000n);
  });
 
  it('alternating error types do not corrupt client state', async () => {
    let call = 0;
    (client as any).invokeContract = async () => {
      call++;
      switch (call) {
        case 1: {
          const e: any = new Error('down');
          e.code = 'ECONNREFUSED';
          throw e;
        }
        case 2:
          throw new Error('Insufficient balance');
        case 3:
          return 100n;
        default:
          return 0n;
      }
    };
 
    await expect(client.getRemainingBalance()).rejects.toThrow(NetworkError);
    await expect(client.getRemainingBalance()).rejects.toThrow(ContractError);
    const balance = await client.getRemainingBalance();
    expect(balance).toBe(100n);
  });
});
 
// =======================================================================
// 7. Edge cases — unusual error shapes handled gracefully
// =======================================================================
describe('Edge-case error shapes', () => {
  let client: ProgramEscrowClient;
  let keypair: Keypair;
 
  beforeEach(() => {
    client = makeClient();
    keypair = Keypair.random();
  });
 
  it('handles error with no message property', async () => {
    stubInvoke(client, { toString: () => 'raw object error' } as any);
    try {
      await client.getProgramInfo();
      fail('Expected an error');
    } catch (err: any) {
      expect(err).toBeInstanceOf(ContractError);
      expect(err.message).toContain('raw object error');
    }
  });
 
  it('handles error that is a plain string', async () => {
    (client as any).invokeContract = async () => {
      throw 'Insufficient balance';
    };
    try {
      await client.getProgramInfo();
      fail('Expected an error');
    } catch (err: any) {
      // String errors go through handleError → parseContractError
      // The string itself has no .code or .response, so it should
      // be parsed as a contract error based on its content.
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe(ContractErrorCode.INSUFFICIENT_BALANCE);
    }
  });
 
  it('handles error with empty message', async () => {
    stubInvoke(client, new Error(''));
    try {
      await client.getProgramInfo();
      fail('Expected an error');
    } catch (err: any) {
      // Empty message → no pattern matches → generic fallback
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe('CONTRACT_ERROR');
    }
  });
 
  it('handles error with only whitespace message', async () => {
    stubInvoke(client, new Error('   '));
    try {
      await client.getProgramInfo();
      fail('Expected an error');
    } catch (err: any) {
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe('CONTRACT_ERROR');
    }
  });
});
