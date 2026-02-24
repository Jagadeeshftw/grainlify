import { ProgramEscrowClient } from '../src/program-escrow-client';
import { Keypair } from '@stellar/stellar-sdk';

/**
 * Example: Lock funds into a program escrow
 */
export async function lockFundsExample(client: ProgramEscrowClient, sourceKeypair: Keypair) {
    const amount = 10000000n; // 10 XLM in stroops
    
    console.log(`Locking ${amount} stroops...`);
    const programData = await client.lockProgramFunds(amount, sourceKeypair);
    console.log('Funds locked successfully.');
    console.log('Remaining balance:', programData.remaining_balance);
    
    return programData;
}
