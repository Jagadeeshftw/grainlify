import { ProgramEscrowClient } from '../program-escrow-client';
import { 
  ContractError, 
  NetworkError, 
  ValidationError, 
  ContractErrorCode,
  createContractError,
  parseContractError
} from '../errors';
import { Keypair } from '@stellar/stellar-sdk';

describe('SDK Client Error Handling', () => {
  let client: ProgramEscrowClient;
  let mockKeypair: Keypair;

  beforeEach(() => {
    client = new ProgramEscrowClient({
      contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
      rpcUrl: 'https://soroban-testnet.stellar.org',
      networkPassphrase: 'Test SDF Network ; September 2015'
    });
    
    mockKeypair = Keypair.random();
  });

  describe('Validation Errors', () => {
    describe('initProgram', () => {
      it('should throw ValidationError for empty program ID', async () => {
        await expect(
          client.initProgram(
            '',
            'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
            'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);

        await expect(
          client.initProgram(
            '',
            'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
            'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
            mockKeypair
          )
        ).rejects.toThrow('Program ID cannot be empty');
      });

      it('should throw ValidationError for invalid authorized payout key address', async () => {
        await expect(
          client.initProgram(
            'test-program',
            'invalid-address',
            'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);

        await expect(
          client.initProgram(
            'test-program',
            'invalid-address',
            'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
            mockKeypair
          )
        ).rejects.toThrow('not a valid Stellar address');
      });

      it('should throw ValidationError for invalid token address', async () => {
        await expect(
          client.initProgram(
            'test-program',
            'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
            'INVALID',
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);
      });

      it('should throw ValidationError for empty addresses', async () => {
        await expect(
          client.initProgram(
            'test-program',
            '',
            'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
            mockKeypair
          )
        ).rejects.toThrow('authorizedPayoutKey cannot be empty');
      });
    });

    describe('lockProgramFunds', () => {
      it('should throw ValidationError for zero amount', async () => {
        await expect(
          client.lockProgramFunds(0n, mockKeypair)
        ).rejects.toThrow(ValidationError);

        await expect(
          client.lockProgramFunds(0n, mockKeypair)
        ).rejects.toThrow('Amount must be greater than zero');
      });

      it('should throw ValidationError for negative amount', async () => {
        await expect(
          client.lockProgramFunds(-100n, mockKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('batchPayout', () => {
      it('should throw ValidationError for empty recipients array', async () => {
        await expect(
          client.batchPayout([], [], mockKeypair)
        ).rejects.toThrow(ValidationError);

        await expect(
          client.batchPayout([], [], mockKeypair)
        ).rejects.toThrow('Recipients array cannot be empty');
      });

      it('should throw ValidationError for mismatched array lengths', async () => {
        const recipients = [
          'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
          'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB'
        ];
        const amounts = [1000n];

        await expect(
          client.batchPayout(recipients, amounts, mockKeypair)
        ).rejects.toThrow(ValidationError);

        await expect(
          client.batchPayout(recipients, amounts, mockKeypair)
        ).rejects.toThrow('same length');
      });

      it('should throw ValidationError for zero or negative amounts', async () => {
        const recipients = [
          'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
          'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB'
        ];
        const amounts = [1000n, 0n];

        await expect(
          client.batchPayout(recipients, amounts, mockKeypair)
        ).rejects.toThrow(ValidationError);

        await expect(
          client.batchPayout(recipients, amounts, mockKeypair)
        ).rejects.toThrow('must be greater than zero');
      });

      it('should throw ValidationError for invalid recipient addresses', async () => {
        const recipients = [
          'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
          'invalid-address'
        ];
        const amounts = [1000n, 2000n];

        await expect(
          client.batchPayout(recipients, amounts, mockKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('singlePayout', () => {
      it('should throw ValidationError for invalid recipient address', async () => {
        await expect(
          client.singlePayout('invalid', 1000n, mockKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('should throw ValidationError for zero amount', async () => {
        await expect(
          client.singlePayout(
            'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
            0n,
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);
      });

      it('should throw ValidationError for negative amount', async () => {
        await expect(
          client.singlePayout(
            'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
            -500n,
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('createProgramReleaseSchedule', () => {
      it('should throw ValidationError for invalid recipient', async () => {
        await expect(
          client.createProgramReleaseSchedule(
            'bad-address',
            1000n,
            Date.now() + 86400,
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);
      });

      it('should throw ValidationError for zero amount', async () => {
        await expect(
          client.createProgramReleaseSchedule(
            'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
            0n,
            Date.now() + 86400,
            mockKeypair
          )
        ).rejects.toThrow(ValidationError);
      });
    });
  });

  describe('Contract Error Parsing', () => {
    it('should parse NOT_INITIALIZED error', () => {
      const error = new Error('Program not initialized');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.NOT_INITIALIZED);
      expect(parsed.message).toContain('not initialized');
    });

    it('should parse UNAUTHORIZED error', () => {
      const error = new Error('require_auth failed');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.UNAUTHORIZED);
    });

    it('should parse INSUFFICIENT_BALANCE error', () => {
      const error = new Error('Insufficient balance');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.INSUFFICIENT_BALANCE);
    });

    it('should parse INVALID_AMOUNT error', () => {
      const error = new Error('Amount must be greater than zero');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.INVALID_AMOUNT);
    });

    it('should parse ALREADY_INITIALIZED error', () => {
      const error = new Error('Program already initialized');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.ALREADY_INITIALIZED);
    });

    it('should parse EMPTY_BATCH error', () => {
      const error = new Error('Cannot process empty batch');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.EMPTY_BATCH);
    });

    it('should parse LENGTH_MISMATCH error', () => {
      const error = new Error('Recipients and amounts vectors must have the same length');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.LENGTH_MISMATCH);
    });

    it('should parse OVERFLOW error', () => {
      const error = new Error('Payout amount overflow');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe(ContractErrorCode.OVERFLOW);
    });

    it('should handle unknown contract errors', () => {
      const error = new Error('Some unknown contract error');
      const parsed = parseContractError(error);
      
      expect(parsed).toBeInstanceOf(ContractError);
      expect(parsed.code).toBe('CONTRACT_ERROR');
      expect(parsed.message).toContain('unknown');
    });
  });

  describe('Contract Error Factory', () => {
    it('should create NOT_INITIALIZED error', () => {
      const error = createContractError(ContractErrorCode.NOT_INITIALIZED);
      
      expect(error).toBeInstanceOf(ContractError);
      expect(error.code).toBe(ContractErrorCode.NOT_INITIALIZED);
      expect(error.message).toBe('Program not initialized');
    });

    it('should create UNAUTHORIZED error with details', () => {
      const error = createContractError(
        ContractErrorCode.UNAUTHORIZED,
        'User ABC123 attempted unauthorized action'
      );
      
      expect(error.code).toBe(ContractErrorCode.UNAUTHORIZED);
      expect(error.message).toContain('Unauthorized');
      expect(error.message).toContain('ABC123');
    });

    it('should create INSUFFICIENT_BALANCE error', () => {
      const error = createContractError(ContractErrorCode.INSUFFICIENT_BALANCE);
      
      expect(error.code).toBe(ContractErrorCode.INSUFFICIENT_BALANCE);
      expect(error.message).toContain('Insufficient balance');
    });

    it('should create all error types correctly', () => {
      const errorCodes = Object.values(ContractErrorCode);
      
      errorCodes.forEach(code => {
        const error = createContractError(code as ContractErrorCode);
        expect(error).toBeInstanceOf(ContractError);
        expect(error.code).toBe(code);
        expect(error.message).toBeTruthy();
      });
    });
  });

  describe('Error Type Hierarchy', () => {
    it('should maintain proper error inheritance', () => {
      const contractError = createContractError(ContractErrorCode.NOT_INITIALIZED);
      const networkError = new NetworkError('Connection failed');
      const validationError = new ValidationError('Invalid input', 'field');

      expect(contractError).toBeInstanceOf(Error);
      expect(networkError).toBeInstanceOf(Error);
      expect(validationError).toBeInstanceOf(Error);
    });

    it('should preserve error properties', () => {
      const contractError = new ContractError('Test error', 'TEST_CODE', 42);
      
      expect(contractError.name).toBe('ContractError');
      expect(contractError.code).toBe('TEST_CODE');
      expect(contractError.contractErrorCode).toBe(42);
      expect(contractError.message).toBe('Test error');
    });

    it('should preserve network error properties', () => {
      const cause = new Error('Original error');
      const networkError = new NetworkError('Network failed', 500, cause);
      
      expect(networkError.name).toBe('NetworkError');
      expect(networkError.code).toBe('NETWORK_ERROR');
      expect(networkError.statusCode).toBe(500);
      expect(networkError.cause).toBe(cause);
    });

    it('should preserve validation error properties', () => {
      const validationError = new ValidationError('Invalid field', 'username');
      
      expect(validationError.name).toBe('ValidationError');
      expect(validationError.code).toBe('VALIDATION_ERROR');
      expect(validationError.field).toBe('username');
    });
  });
});
