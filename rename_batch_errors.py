import os

def rename_batch_errors(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # BatchError::InvalidBatchSize -> BatchError::InvalidBatchSizeProgram
    # BatchError::DuplicateEntry -> BatchError::DuplicateProgramId
    
    content = content.replace('BatchError::InvalidBatchSize', 'BatchError::InvalidBatchSizeProgram')
    content = content.replace('BatchError::DuplicateEntry', 'BatchError::DuplicateProgramId')
    
    with open(file_path, 'w') as f:
        f.write(content)
    return True

test_files = [
    'contracts/program-escrow/src/test.rs',
    'contracts/program-escrow/src/test_pause.rs',
    'contracts/program-escrow/src/test_allowance.rs',
    'contracts/program-escrow/src/test_batch_operations.rs',
    'contracts/program-escrow/src/test_granular_pause.rs',
    'contracts/program-escrow/src/test_lifecycle.rs',
    'contracts/program-escrow/src/test_maintenance_mode.rs',
    'contracts/program-escrow/src/test_deterministic_error_ordering.rs',
    'contracts/program-escrow/src/test_dispute_resolution.rs',
    'contracts/program-escrow/src/test_reputation.rs',
    'contracts/program-escrow/src/test_read_only_mode.rs',
    'contracts/program-escrow/src/test_claim_period_expiry_cancellation.rs',
    'contracts/program-escrow/src/test_time_weighted_metrics.rs',
    'contracts/program-escrow/src/test_circuit_breaker_audit.rs'
]

for f in test_files:
    if os.path.exists(f):
        changed = rename_batch_errors(f)
        print(f"{f}: {'Changed' if changed else 'No change'}")
