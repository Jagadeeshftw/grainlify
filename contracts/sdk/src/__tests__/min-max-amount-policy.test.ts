import { ProgramEscrowClient } from '../program-escrow-client';
import {
  ValidationError,
  ContractError,
  ContractErrorCode,
  createContractError,
  parseContractError,
} from '../errors';
import { Keypair } from '@stellar/stellar-sdk';

// ── Min/Max Amount Policy Enforcement – SDK Tests ────────────────────────────
//
// Covers Issue #375: SDK-level validation and error mapping for configurable
// min/max amount limits introduced in Issue #62.
//
// Tests are split into two layers:
//   1. Client-side validation (ValidationError) — rejected before any network
//      call, so no keypair / RPC interaction is needed.
//   2. Contract-error mapping — verifies that error strings returned by the
//      on-chain contract (Issue #62) are parsed into the correct SDK error
//      codes by parseContractError / createContractError.
// ─────────────────────────────────────────────────────────────────────────────

const VALID_ADDRESS_A = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
const VALID_ADDRESS_B = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';

describe('Min/Max Amount Policy Enforcement', () => {
  let client: ProgramEscrowClient;
  let mockKeypair: Keypair;

  beforeEach(() => {
    client = new ProgramEscrowClient({
      contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
      rpcUrl: 'https://soroban-testnet.stellar.org',
      networkPassphrase: 'Test SDF Network ; September 2015',
    });
    mockKeypair = Keypair.random();
  });

  // ── lockProgramFunds – client-side amount validation ──────────────────────

  describe('lockProgramFunds – amount boundary validation', () => {
    it('rejects zero amount with ValidationError before reaching the contract', async () => {
      await expect(client.lockProgramFunds(0n, mockKeypair))
        .rejects.toBeInstanceOf(ValidationError);

      await expect(client.lockProgramFunds(0n, mockKeypair))
        .rejects.toThrow('Amount must be greater than zero');
    });

    it('rejects negative amount with ValidationError before reaching the contract', async () => {
      await expect(client.lockProgramFunds(-1n, mockKeypair))
        .rejects.toBeInstanceOf(ValidationError);

      await expect(client.lockProgramFunds(-9_999n, mockKeypair))
        .rejects.toBeInstanceOf(ValidationError);
    });

    it('surfaces AMOUNT_BELOW_MIN when the contract rejects a value below the policy minimum', () => {
      // Simulates the error string emitted by the on-chain contract (Issue #62).
      const contractReply = new Error('Amount below minimum allowed');
      const parsed = parseContractError(contractReply);

      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_BELOW_MIN);
      expect(parsed.message).toMatch(/below.*min(imum)?/i);
    });

    it('surfaces AMOUNT_ABOVE_MAX when the contract rejects a value above the policy maximum', () => {
      const contractReply = new Error('Amount exceeds maximum allowed');
      const parsed = parseContractError(contractReply);

      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_ABOVE_MAX);
      expect(parsed.message).toMatch(/above.*max(imum)?|exceed.*max/i);
    });
  });

  // ── singlePayout – client-side and contract-error validation ─────────────

  describe('singlePayout – amount boundary validation', () => {
    it('rejects zero payout amount with ValidationError', async () => {
      await expect(client.singlePayout(VALID_ADDRESS_A, 0n, mockKeypair))
        .rejects.toBeInstanceOf(ValidationError);
    });

    it('rejects negative payout amount with ValidationError', async () => {
      await expect(client.singlePayout(VALID_ADDRESS_A, -500n, mockKeypair))
        .rejects.toBeInstanceOf(ValidationError);
    });

    it('surfaces AMOUNT_BELOW_MIN error from contract for payout below minimum', () => {
      const contractReply = new Error('Amount below minimum allowed');
      const parsed = parseContractError(contractReply);

      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_BELOW_MIN);
    });

    it('surfaces AMOUNT_ABOVE_MAX error from contract for payout above maximum', () => {
      const contractReply = new Error('Amount exceeds maximum allowed');
      const parsed = parseContractError(contractReply);

      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_ABOVE_MAX);
    });
  });

  // ── batchPayout – per-item validation ────────────────────────────────────

  describe('batchPayout – per-item amount boundary validation', () => {
    it('rejects a batch where any single amount is zero', async () => {
      await expect(
        client.batchPayout(
          [VALID_ADDRESS_A, VALID_ADDRESS_B],
          [1_000n, 0n],
          mockKeypair,
        ),
      ).rejects.toBeInstanceOf(ValidationError);
    });

    it('rejects a batch where any single amount is negative', async () => {
      await expect(
        client.batchPayout(
          [VALID_ADDRESS_A, VALID_ADDRESS_B],
          [-1n, 1_000n],
          mockKeypair,
        ),
      ).rejects.toBeInstanceOf(ValidationError);
    });

    it('surfaces AMOUNT_BELOW_MIN for batch items below the policy minimum', () => {
      const contractReply = new Error('Amount below minimum allowed');
      const parsed = parseContractError(contractReply);

      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_BELOW_MIN);
    });

    it('surfaces AMOUNT_ABOVE_MAX for batch items above the policy maximum', () => {
      const contractReply = new Error('Amount exceeds maximum allowed');
      const parsed = parseContractError(contractReply);

      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_ABOVE_MAX);
    });
  });

  // ── createContractError – new policy error codes ─────────────────────────

  describe('createContractError – new policy error codes', () => {
    it('creates AMOUNT_BELOW_MIN error with a non-empty message', () => {
      const error = createContractError(ContractErrorCode.AMOUNT_BELOW_MIN);

      expect(error).toBeInstanceOf(ContractError);
      expect(error.code).toBe(ContractErrorCode.AMOUNT_BELOW_MIN);
      expect(error.message).toBeTruthy();
    });

    it('creates AMOUNT_ABOVE_MAX error with a non-empty message', () => {
      const error = createContractError(ContractErrorCode.AMOUNT_ABOVE_MAX);

      expect(error).toBeInstanceOf(ContractError);
      expect(error.code).toBe(ContractErrorCode.AMOUNT_ABOVE_MAX);
      expect(error.message).toBeTruthy();
    });

    it('appends optional detail string to AMOUNT_BELOW_MIN message', () => {
      const error = createContractError(
        ContractErrorCode.AMOUNT_BELOW_MIN,
        'minimum is 100 stroops',
      );

      expect(error.message).toContain('100 stroops');
    });

    it('appends optional detail string to AMOUNT_ABOVE_MAX message', () => {
      const error = createContractError(
        ContractErrorCode.AMOUNT_ABOVE_MAX,
        'maximum is 10000 stroops',
      );

      expect(error.message).toContain('10000 stroops');
    });
  });

  // ── parseContractError – exhaustive message-to-code mapping ──────────────

  describe('parseContractError – policy error message variants', () => {
    // All message variants the on-chain contract (Issue #62) may emit for a
    // below-minimum violation.
    const belowMinMessages = [
      'Amount below minimum allowed',
      'amount is below the minimum',
      'AmountBelowMinimum',
      'below min',
    ];

    // All message variants for an above-maximum violation.
    const aboveMaxMessages = [
      'Amount exceeds maximum allowed',
      'amount exceeds maximum',
      'AmountAboveMaximum',
      'above max',
    ];

    belowMinMessages.forEach((msg) => {
      it(`maps "${msg}" → AMOUNT_BELOW_MIN`, () => {
        const parsed = parseContractError(new Error(msg));
        expect(parsed.code).toBe(ContractErrorCode.AMOUNT_BELOW_MIN);
      });
    });

    aboveMaxMessages.forEach((msg) => {
      it(`maps "${msg}" → AMOUNT_ABOVE_MAX`, () => {
        const parsed = parseContractError(new Error(msg));
        expect(parsed.code).toBe(ContractErrorCode.AMOUNT_ABOVE_MAX);
      });
    });

    // Ensure the more precise min/max codes take precedence over INVALID_AMOUNT.
    it('prefers AMOUNT_BELOW_MIN over INVALID_AMOUNT for below-minimum errors', () => {
      // Message that could naively match the generic "greater than zero" check.
      const parsed = parseContractError(new Error('Amount below minimum allowed'));
      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_BELOW_MIN);
      expect(parsed.code).not.toBe(ContractErrorCode.INVALID_AMOUNT);
    });

    it('prefers AMOUNT_ABOVE_MAX over INVALID_AMOUNT for above-maximum errors', () => {
      const parsed = parseContractError(new Error('Amount exceeds maximum allowed'));
      expect(parsed.code).toBe(ContractErrorCode.AMOUNT_ABOVE_MAX);
      expect(parsed.code).not.toBe(ContractErrorCode.INVALID_AMOUNT);
    });
  });
});