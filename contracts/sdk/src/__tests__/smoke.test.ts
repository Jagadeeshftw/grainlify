import { ProgramEscrowClient } from '../program-escrow-client';
import { Keypair } from '@stellar/stellar-sdk';
import { lockFundsExample } from '../../examples/lock-funds';
import { releaseFundsExample } from '../../examples/release-funds';
import { fullLifecycleExample } from '../../examples/full-lifecycle';
import { batchLockExample } from '../../examples/batch-lock';
import { queryEscrowExample } from '../../examples/query-escrow';

// Mock the console methods to keep test output clean
jest.spyOn(console, 'log').mockImplementation(() => { });
jest.spyOn(process.stdout, 'write').mockImplementation(() => true);

describe('SDK Example Smoke Tests', () => {
    let client: ProgramEscrowClient;
    let mockKeypair: Keypair;
    const mockProgramId = 'test-program-123';
    const mockAuthorizedKey = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
    const mockTokenAddress = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';

    beforeEach(() => {
        client = new ProgramEscrowClient({
            contractId: 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
            rpcUrl: 'https://soroban-testnet.stellar.org',
            networkPassphrase: 'Test SDF Network ; September 2015'
        });

        mockKeypair = Keypair.random();

        // Mock invokeContract to simulate successful Soroban interactions
        // @ts-ignore - accessing private method for testing purposes
        jest.spyOn(client, 'invokeContract').mockImplementation(async (method: string, args: any[]) => {
            if (method === 'get_program_info' || method === 'init_program' || method === 'lock_program_funds' || method === 'batch_payout') {
                return {
                    program_id: mockProgramId,
                    total_funds: 100000000n,
                    remaining_balance: 50000000n,
                    authorized_payout_key: mockAuthorizedKey,
                    payout_history: [],
                    token_address: mockTokenAddress
                };
            }
            if (method === 'trigger_program_releases') {
                return 3n; // Simulate 3 releases triggered
            }
            return null;
        });
    });

    afterEach(() => {
        jest.clearAllMocks();
    });

    it('should run lock-funds example successfully', async () => {
        const result = await lockFundsExample(client, mockKeypair);
        expect(result).toBeDefined();
        //@ts-ignore - accessing private field
        expect(client.invokeContract).toHaveBeenCalledWith('lock_program_funds', [10000000n], mockKeypair);
    });

    it('should run release-funds example successfully', async () => {
        const result = await releaseFundsExample(client, mockKeypair);
        expect(result).toBe(3);
        //@ts-ignore - accessing private field
        expect(client.invokeContract).toHaveBeenCalledWith('trigger_program_releases', [], mockKeypair);
    });

    it('should run full-lifecycle example successfully', async () => {
        const result = await fullLifecycleExample(
            client,
            mockKeypair,
            mockProgramId,
            mockAuthorizedKey,
            mockTokenAddress
        );
        expect(result).toBeDefined();
        //@ts-ignore - accessing private field
        expect(client.invokeContract).toHaveBeenCalledWith('init_program', expect.anything(), mockKeypair);
    });

    it('should run batch-lock example successfully', async () => {
        const result = await batchLockExample(client, mockKeypair);
        expect(result).toBeDefined();
        //@ts-ignore - accessing private field
        expect(client.invokeContract).toHaveBeenCalledWith('lock_program_funds', expect.anything(), mockKeypair);
    });

    it('should run query-escrow example successfully', async () => {
        const result = await queryEscrowExample(client);
        expect(result).toBeDefined();
        expect(result.program_id).toBe(mockProgramId);
        //@ts-ignore - accessing private field
        expect(client.invokeContract).toHaveBeenCalledWith('get_program_info', []);
    });
});
