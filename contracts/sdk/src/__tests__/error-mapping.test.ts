/**
 * Tests that verify every contract error code is mapped to a human-readable
 * message in the SDK, and that the numeric look-up tables are complete.
 *
 * These tests act as a safety net: if a new error variant is added to a
 * contract but not reflected here, the "completeness" assertions will fail.
 */
import {
  ContractErrorCode,
  ContractError,
  createContractError,
  parseContractError,
  parseContractErrorByCode,
  getContractErrorMessage,
  BOUNTY_ESCROW_ERROR_MAP,
  GOVERNANCE_ERROR_MAP,
  CIRCUIT_BREAKER_ERROR_MAP,
} from '../errors';

// -----------------------------------------------------------------------
// Authoritative list of every error discriminant in each contract.
// Sourced directly from the Rust source files — keep in sync.
// -----------------------------------------------------------------------

/** contracts/bounty_escrow/contracts/escrow/src/lib.rs — Error enum */
const BOUNTY_ESCROW_DISCRIMINANTS: number[] = [
  1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, /* gap at 15 */ 16, 17, 18,
];

/** contracts/grainlify-core/src/governance.rs — Error enum */
const GOVERNANCE_DISCRIMINANTS: number[] = [
  1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
];

/** contracts/program-escrow/src/error_recovery.rs — u32 constants */
const CIRCUIT_BREAKER_CODES: number[] = [1001, 1002, 1003];

// =======================================================================
// 1. Completeness: every ContractErrorCode has a non-empty message
// =======================================================================
describe('Error mapping completeness', () => {
  const allCodes = Object.values(ContractErrorCode);

  it('every ContractErrorCode has a human-readable message', () => {
    for (const code of allCodes) {
      const msg = getContractErrorMessage(code as ContractErrorCode);
      expect(msg).toBeDefined();
      expect(msg.length).toBeGreaterThan(0);
    }
  });

  it('createContractError produces a ContractError for every code', () => {
    for (const code of allCodes) {
      const err = createContractError(code as ContractErrorCode);
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe(code);
      expect(err.message).toBeTruthy();
    }
  });

  it('createContractError appends details when provided', () => {
    for (const code of allCodes) {
      const err = createContractError(code as ContractErrorCode, 'extra context');
      expect(err.message).toContain('extra context');
    }
  });
});

// =======================================================================
// 2. Numeric look-up tables cover every on-chain discriminant
// =======================================================================
describe('Numeric error code tables', () => {
  describe('Bounty-escrow', () => {
    it('maps every contract discriminant (1-18, excluding 15)', () => {
      for (const code of BOUNTY_ESCROW_DISCRIMINANTS) {
        expect(BOUNTY_ESCROW_ERROR_MAP[code]).toBeDefined();
      }
    });

    it('resolves via parseContractErrorByCode', () => {
      for (const code of BOUNTY_ESCROW_DISCRIMINANTS) {
        const err = parseContractErrorByCode(code, 'bounty_escrow');
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).not.toBe('CONTRACT_ERROR');
        expect(err.contractErrorCode).toBe(code);
      }
    });

    it('returns generic error for unmapped code', () => {
      const err = parseContractErrorByCode(15, 'bounty_escrow');
      expect(err.code).toBe('CONTRACT_ERROR');
      expect(err.contractErrorCode).toBe(15);
      expect(err.message).toContain('Unknown');
    });
  });

  describe('Governance', () => {
    it('maps every contract discriminant (1-14)', () => {
      for (const code of GOVERNANCE_DISCRIMINANTS) {
        expect(GOVERNANCE_ERROR_MAP[code]).toBeDefined();
      }
    });

    it('resolves via parseContractErrorByCode', () => {
      for (const code of GOVERNANCE_DISCRIMINANTS) {
        const err = parseContractErrorByCode(code, 'governance');
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).not.toBe('CONTRACT_ERROR');
        expect(err.contractErrorCode).toBe(code);
      }
    });

    it('returns generic error for unmapped code', () => {
      const err = parseContractErrorByCode(99, 'governance');
      expect(err.code).toBe('CONTRACT_ERROR');
      expect(err.contractErrorCode).toBe(99);
    });
  });

  describe('Circuit-breaker', () => {
    it('maps every error constant (1001-1003)', () => {
      for (const code of CIRCUIT_BREAKER_CODES) {
        expect(CIRCUIT_BREAKER_ERROR_MAP[code]).toBeDefined();
      }
    });

    it('resolves via parseContractErrorByCode', () => {
      for (const code of CIRCUIT_BREAKER_CODES) {
        const err = parseContractErrorByCode(code, 'circuit_breaker');
        expect(err).toBeInstanceOf(ContractError);
        expect(err.code).not.toBe('CONTRACT_ERROR');
        expect(err.contractErrorCode).toBe(code);
      }
    });

    it('returns generic error for unmapped code', () => {
      const err = parseContractErrorByCode(9999, 'circuit_breaker');
      expect(err.code).toBe('CONTRACT_ERROR');
    });
  });
});

// =======================================================================
// 3. String-based parseContractError covers representative patterns
// =======================================================================
describe('parseContractError string matching', () => {
  // ── Program-escrow ──────────────────────────────────────────────────
  const programEscrowCases: [string, ContractErrorCode][] = [
    ['Program not initialized',                        ContractErrorCode.NOT_INITIALIZED],
    ['require_auth failed',                            ContractErrorCode.UNAUTHORIZED],
    ['Insufficient balance',                           ContractErrorCode.INSUFFICIENT_BALANCE],
    ['Amount must be greater than zero',               ContractErrorCode.INVALID_AMOUNT],
    ['Program already initialized',                    ContractErrorCode.ALREADY_INITIALIZED],
    ['Cannot process empty batch',                     ContractErrorCode.EMPTY_BATCH],
    ['Recipients and amounts must have the same length', ContractErrorCode.LENGTH_MISMATCH],
    ['Payout amount overflow',                         ContractErrorCode.OVERFLOW],
    ['Amount is below minimum',                        ContractErrorCode.AMOUNT_BELOW_MIN],
    ['AmountBelowMinimum',                             ContractErrorCode.AMOUNT_BELOW_MIN],
    ['Amount exceeds maximum allowed',                 ContractErrorCode.AMOUNT_ABOVE_MAX],
    ['AmountAboveMaximum',                             ContractErrorCode.AMOUNT_ABOVE_MAX],
  ];

  it.each(programEscrowCases)(
    'program-escrow: "%s" → %s',
    (message, expectedCode) => {
      const err = parseContractError(new Error(message));
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe(expectedCode);
    },
  );

  // ── Bounty-escrow ──────────────────────────────────────────────────
  const bountyEscrowCases: [string, ContractErrorCode][] = [
    ['BountyExists',                                   ContractErrorCode.BOUNTY_EXISTS],
    ['Bounty not found',                               ContractErrorCode.BOUNTY_NOT_FOUND],
    ['FundsNotLocked',                                 ContractErrorCode.BOUNTY_FUNDS_NOT_LOCKED],
    ['DeadlineNotPassed',                              ContractErrorCode.BOUNTY_DEADLINE_NOT_PASSED],
    ['InvalidFeeRate',                                 ContractErrorCode.BOUNTY_INVALID_FEE_RATE],
    ['Fee recipient address not set',                  ContractErrorCode.BOUNTY_FEE_RECIPIENT_NOT_SET],
    ['InvalidBatchSize',                               ContractErrorCode.BOUNTY_INVALID_BATCH_SIZE],
    ['BatchSizeMismatch',                              ContractErrorCode.BOUNTY_BATCH_SIZE_MISMATCH],
    ['DuplicateBountyId',                              ContractErrorCode.BOUNTY_DUPLICATE_ID],
    ['Bounty amount is invalid',                       ContractErrorCode.BOUNTY_INVALID_AMOUNT],
    ['Bounty deadline is invalid',                     ContractErrorCode.BOUNTY_INVALID_DEADLINE],
    ['InsufficientFunds',                              ContractErrorCode.BOUNTY_INSUFFICIENT_FUNDS],
    ['RefundNotApproved',                              ContractErrorCode.BOUNTY_REFUND_NOT_APPROVED],
    ['FundsPaused',                                    ContractErrorCode.BOUNTY_FUNDS_PAUSED],
  ];

  it.each(bountyEscrowCases)(
    'bounty-escrow: "%s" → %s',
    (message, expectedCode) => {
      const err = parseContractError(new Error(message));
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe(expectedCode);
    },
  );

  // ── Governance ─────────────────────────────────────────────────────
  const governanceCases: [string, ContractErrorCode][] = [
    ['ProposalNotFound',                               ContractErrorCode.GOV_PROPOSAL_NOT_FOUND],
    ['ProposalNotActive',                              ContractErrorCode.GOV_PROPOSAL_NOT_ACTIVE],
    ['VotingNotStarted',                               ContractErrorCode.GOV_VOTING_NOT_STARTED],
    ['VotingEnded',                                    ContractErrorCode.GOV_VOTING_ENDED],
    ['VotingStillActive',                              ContractErrorCode.GOV_VOTING_STILL_ACTIVE],
    ['AlreadyVoted',                                   ContractErrorCode.GOV_ALREADY_VOTED],
    ['ProposalNotApproved',                            ContractErrorCode.GOV_PROPOSAL_NOT_APPROVED],
    ['ExecutionDelayNotMet',                           ContractErrorCode.GOV_EXECUTION_DELAY_NOT_MET],
    ['ProposalExpired',                                ContractErrorCode.GOV_PROPOSAL_EXPIRED],
    ['InsufficientStake',                              ContractErrorCode.GOV_INSUFFICIENT_STAKE],
    ['InvalidThreshold',                               ContractErrorCode.GOV_INVALID_THRESHOLD],
    ['ThresholdTooLow',                                ContractErrorCode.GOV_THRESHOLD_TOO_LOW],
  ];

  it.each(governanceCases)(
    'governance: "%s" → %s',
    (message, expectedCode) => {
      const err = parseContractError(new Error(message));
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe(expectedCode);
    },
  );

  // ── Circuit-breaker ────────────────────────────────────────────────
  const circuitBreakerCases: [string, ContractErrorCode][] = [
    ['Circuit breaker is open',                        ContractErrorCode.CIRCUIT_OPEN],
    ['Token transfer failed',                          ContractErrorCode.CIRCUIT_TRANSFER_FAILED],
  ];

  it.each(circuitBreakerCases)(
    'circuit-breaker: "%s" → %s',
    (message, expectedCode) => {
      const err = parseContractError(new Error(message));
      expect(err).toBeInstanceOf(ContractError);
      expect(err.code).toBe(expectedCode);
    },
  );

  // ── Fallback ───────────────────────────────────────────────────────
  it('returns generic CONTRACT_ERROR for unrecognised messages', () => {
    const err = parseContractError(new Error('something completely different'));
    expect(err).toBeInstanceOf(ContractError);
    expect(err.code).toBe('CONTRACT_ERROR');
    expect(err.message).toContain('something completely different');
  });

  it('handles null/undefined input gracefully', () => {
    const err = parseContractError(null);
    expect(err).toBeInstanceOf(ContractError);
    expect(err.code).toBe('CONTRACT_ERROR');
  });

  it('handles string input', () => {
    const err = parseContractError('Bounty not found');
    expect(err).toBeInstanceOf(ContractError);
    expect(err.code).toBe(ContractErrorCode.BOUNTY_NOT_FOUND);
  });
});

// =======================================================================
// 4. Cross-layer consistency: numeric ↔ string resolution agrees
// =======================================================================
describe('Cross-layer consistency', () => {
  it('bounty-escrow numeric and string parsers yield the same code', () => {
    const numericToString: [number, string][] = [
      [3,  'BountyExists'],
      [4,  'Bounty not found'],
      [13, 'Bounty amount is invalid'],
      [16, 'InsufficientFunds'],
    ];

    for (const [code, message] of numericToString) {
      const fromNumeric = parseContractErrorByCode(code, 'bounty_escrow');
      const fromString = parseContractError(new Error(message));
      expect(fromNumeric.code).toBe(fromString.code);
    }
  });

  it('governance numeric and string parsers yield the same code', () => {
    const numericToString: [number, string][] = [
      [6,  'ProposalNotFound'],
      [9,  'VotingEnded'],
      [11, 'AlreadyVoted'],
      [14, 'ProposalExpired'],
    ];

    for (const [code, message] of numericToString) {
      const fromNumeric = parseContractErrorByCode(code, 'governance');
      const fromString = parseContractError(new Error(message));
      expect(fromNumeric.code).toBe(fromString.code);
    }
  });
});

// =======================================================================
// 5. Regression guard: expected enum counts
// =======================================================================
describe('Enum size regression guards', () => {
  it('ContractErrorCode has the expected number of values', () => {
    const count = Object.keys(ContractErrorCode).length;
    // 10 program-escrow + 17 bounty-escrow + 14 governance + 3 circuit-breaker = 44
    expect(count).toBe(44);
  });

  it('BOUNTY_ESCROW_ERROR_MAP has 17 entries', () => {
    expect(Object.keys(BOUNTY_ESCROW_ERROR_MAP).length).toBe(17);
  });

  it('GOVERNANCE_ERROR_MAP has 14 entries', () => {
    expect(Object.keys(GOVERNANCE_ERROR_MAP).length).toBe(14);
  });

  it('CIRCUIT_BREAKER_ERROR_MAP has 3 entries', () => {
    expect(Object.keys(CIRCUIT_BREAKER_ERROR_MAP).length).toBe(3);
  });
});
