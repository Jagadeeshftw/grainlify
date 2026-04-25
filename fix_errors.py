import os
import re

def fix_contract_error_refs(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Pattern: grainlify_core::errors::ContractError::SomeError
    # should be BatchError::SomeError
    
    new_content = content.replace('grainlify_core::errors::ContractError::', 'BatchError::')
    
    if new_content != content:
        with open(file_path, 'w') as f:
            f.write(new_content)
        return True
    return False

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
        changed = fix_contract_error_refs(f)
        print(f"{f}: {'Changed' if changed else 'No change'}")
