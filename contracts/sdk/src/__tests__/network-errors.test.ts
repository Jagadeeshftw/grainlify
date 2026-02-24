import { ProgramEscrowClient } from '../program-escrow-client';
import { NetworkError } from '../errors';
import { Keypair } from '@stellar/stellar-sdk';

describe('SDK Network Error Handling', () => {
  let mockKeypair: Keypair;

  beforeEach(() => {
    mockKeypair = Keypair.random();
  });

  describe('Connection Errors', () => {
    it('should handle connection refused errors', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'http://localhost:9999', // Non-existent server
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      // Mock the invokeContract to simulate connection error
      const originalInvoke = (client as any).invokeContract;
      (client as any).invokeContract = async () => {
        const error: any = new Error('Connection refused');
        error.code = 'ECONNREFUSED';
        throw error;
      };

      await expect(
        client.getProgramInfo()
      ).rejects.toThrow(NetworkError);

      await expect(
        client.getProgramInfo()
      ).rejects.toThrow('Failed to connect to RPC server');
    });

    it('should handle timeout errors', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Request timeout');
        error.code = 'ETIMEDOUT';
        throw error;
      };

      await expect(
        client.getRemainingBalance()
      ).rejects.toThrow(NetworkError);
    });

    it('should handle DNS resolution errors', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://nonexistent-domain-12345.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('getaddrinfo ENOTFOUND');
        error.code = 'ENOTFOUND';
        throw error;
      };

      await expect(
        client.getProgramInfo()
      ).rejects.toThrow();
    });
  });

  describe('HTTP Status Errors', () => {
    it('should handle 400 Bad Request', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Bad Request');
        error.response = { status: 400 };
        throw error;
      };

      await expect(
        client.lockProgramFunds(1000n, mockKeypair)
      ).rejects.toThrow(NetworkError);

      try {
        await client.lockProgramFunds(1000n, mockKeypair);
      } catch (error: any) {
        expect(error.statusCode).toBe(400);
      }
    });

    it('should handle 401 Unauthorized', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Unauthorized');
        error.response = { status: 401 };
        throw error;
      };

      await expect(
        client.singlePayout(
          'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
          1000n,
          mockKeypair
        )
      ).rejects.toThrow(NetworkError);
    });

    it('should handle 404 Not Found', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Not Found');
        error.response = { status: 404 };
        throw error;
      };

      await expect(
        client.getProgramInfo()
      ).rejects.toThrow(NetworkError);
    });

    it('should handle 500 Internal Server Error', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Internal Server Error');
        error.response = { status: 500 };
        throw error;
      };

      await expect(
        client.getRemainingBalance()
      ).rejects.toThrow(NetworkError);

      try {
        await client.getRemainingBalance();
      } catch (error: any) {
        expect(error.statusCode).toBe(500);
        expect(error.message).toContain('500');
      }
    });

    it('should handle 503 Service Unavailable', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Service Unavailable');
        error.response = { status: 503 };
        throw error;
      };

      await expect(
        client.triggerProgramReleases(mockKeypair)
      ).rejects.toThrow(NetworkError);
    });
  });

  describe('Network Error Properties', () => {
    it('should preserve cause in NetworkError', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      const originalError = new Error('Original network error');
      (originalError as any).code = 'ECONNREFUSED';

      (client as any).invokeContract = async () => {
        throw originalError;
      };

      try {
        await client.getProgramInfo();
        fail('Should have thrown NetworkError');
      } catch (error: any) {
        expect(error).toBeInstanceOf(NetworkError);
        expect(error.cause).toBe(originalError);
      }
    });

    it('should include RPC URL in error message', async () => {
      const rpcUrl = 'https://custom-rpc.example.com';
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl,
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      (client as any).invokeContract = async () => {
        const error: any = new Error('Connection failed');
        error.code = 'ECONNREFUSED';
        throw error;
      };

      try {
        await client.getProgramInfo();
        fail('Should have thrown NetworkError');
      } catch (error: any) {
        expect(error.message).toContain(rpcUrl);
      }
    });
  });

  describe('Error Recovery', () => {
    it('should allow retry after network error', async () => {
      const client = new ProgramEscrowClient({
        contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
        rpcUrl: 'https://soroban-testnet.stellar.org',
        networkPassphrase: 'Test SDF Network ; September 2015'
      });

      let callCount = 0;
      (client as any).invokeContract = async () => {
        callCount++;
        if (callCount === 1) {
          const error: any = new Error('Temporary failure');
          error.response = { status: 503 };
          throw error;
        }
        return 5000n; // Success on second call
      };

      // First call fails
      await expect(client.getRemainingBalance()).rejects.toThrow(NetworkError);
      
      // Second call succeeds
      const balance = await client.getRemainingBalance();
      expect(balance).toBe(5000n);
      expect(callCount).toBe(2);
    });
  });
});
