import os
import re

def fix_publish_program(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Pattern: client.init_program(program_id_expr, ...)
    # and we want to find the following .publish_program()
    
    # We'll do a line-by-line pass with a state machine for each file.
    lines = content.split('\n')
    new_lines = []
    
    # We'll track the last seen program_id expression for each client variable
    last_program_ids = {} # {client_var: id_expr}
    
    for i, line in enumerate(lines):
        # Match init_program call: client.init_program(ID, ...)
        init_match = re.search(r'(\w+)\.init_program\(\s*&?([^,]+),', line)
        if init_match:
            client_var = init_match.group(1)
            id_expr = init_match.group(2).strip()
            last_program_ids[client_var] = id_expr
            
        # Also match init_program_with_metadata, etc.
        init_match_v2 = re.search(r'(\w+)\.init_program_with_metadata\(\s*&?([^,]+),', line)
        if init_match_v2:
            client_var = init_match_v2.group(1)
            id_expr = init_match_v2.group(2).strip()
            last_program_ids[client_var] = id_expr

        # Match publish_program() call
        publish_match = re.search(r'(\w+)\.(?:try_)?publish_program\(\)', line)
        if publish_match:
            client_var = publish_match.group(1)
            id_expr = last_program_ids.get(client_var)
            
            # Special case for helpers where program_id is a parameter
            if not id_expr and "program_id" in content:
                # Check if we are inside a function that has program_id as parameter
                # This is a bit heuristic
                if "program_id" in content: id_expr = "program_id"

            if id_expr:
                # If id_expr starts with & and we are passing to publish_program which takes String, 
                # we might need to be careful, but Soroban clients usually take &String.
                line = line.replace('.publish_program()', f'.publish_program(&{id_expr})')
                line = line.replace('.try_publish_program()', f'.try_publish_program(&{id_expr})')
        
        new_lines.append(line)
        
    new_content = '\n'.join(new_lines)
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
        changed = fix_publish_program(f)
        print(f"{f}: {'Changed' if changed else 'No change'}")
