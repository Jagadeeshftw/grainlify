import { ProgramEscrowClient } from '../src/program-escrow-client';

/**
 * Example: Query escrow information
 */
export async function queryEscrowExample(client: ProgramEscrowClient) {
    console.log('Querying program information...');
    const info = await client.getProgramInfo();

    console.log('--- Program Escrow Status ---');
    console.log('Program ID:', info.program_id);
    console.log('Token Address:', info.token_address);
    console.log('Total Funds:', info.total_funds);
    console.log('Remaining Balance:', info.remaining_balance);
    console.log('Authorized Payout Key:', info.authorized_payout_key);
    console.log('Payout History Count:', info.payout_history.length);

    return info;
}
