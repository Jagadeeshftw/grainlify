import { ProgramEscrowClient } from '../src/program-escrow-client';
import { Keypair } from '@stellar/stellar-sdk';

/**
 * Example: Sequential fund locking (Batch Lock)
 */
export async function batchLockExample(client: ProgramEscrowClient, sourceKeypair: Keypair) {
    const lockAmounts = [10000000n, 20000000n, 30000000n];

    console.log(`Executing ${lockAmounts.length} sequential locks...`);

    for (const amount of lockAmounts) {
        process.stdout.write(`Locking ${amount} stroops... `);
        await client.lockProgramFunds(amount, sourceKeypair);
        console.log('Done.');
    }

    const finalInfo = await client.getProgramInfo();
    console.log('Total funds after batch locks:', finalInfo.total_funds);

    return finalInfo;
}
