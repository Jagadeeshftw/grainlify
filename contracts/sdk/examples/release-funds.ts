import { ProgramEscrowClient } from '../src/program-escrow-client';
import { Keypair } from '@stellar/stellar-sdk';

/**
 * Example: Trigger program releases
 */
export async function releaseFundsExample(client: ProgramEscrowClient, sourceKeypair: Keypair) {
    console.log('Triggering program releases...');
    const releasedCount = await client.triggerProgramReleases(sourceKeypair);
    console.log(`Successfully triggered ${releasedCount} releases.`);

    return releasedCount;
}
