import { ProgramEscrowClient } from '../src/program-escrow-client';
import { Keypair } from '@stellar/stellar-sdk';

/**
 * Example: Full program lifecycle
 */
export async function fullLifecycleExample(
    client: ProgramEscrowClient,
    sourceKeypair: Keypair,
    programId: string,
    authorizedPayoutKey: string,
    tokenAddress: string
) {
    console.log('--- Step 1: Initialize Program ---');
    const initData = await client.initProgram(
        programId,
        authorizedPayoutKey,
        tokenAddress,
        sourceKeypair,
        50000000n
    );
    console.log('Program initialized:', initData.program_id);

    console.log('--- Step 2: Publish Program ---');
    await client.publishProgram(programId, sourceKeypair.publicKey(), sourceKeypair);
    console.log('Program published.');

    console.log('--- Step 3: Batch Payout ---');
    // Both recipients are valid, funded network accounts supplied by the
    // caller. This keeps the example runnable on an actual deployment.
    const recipients = [sourceKeypair.publicKey(), sourceKeypair.publicKey()];
    const amounts = [10000000n, 15000000n];
    await client.batchPayout(recipients, amounts, sourceKeypair);
    console.log('Batch payout executed.');

    console.log('--- Step 4: Get Final Info ---');
    const finalInfo = await client.getProgramInfo();
    console.log('Final remaining balance:', finalInfo.remaining_balance);

    return finalInfo;
}
