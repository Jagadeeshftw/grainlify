import os
import re

def fix_stale_tests(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Fix rotate_payout_key calls: 4 args -> 2 args
    # client.rotate_payout_key(&program_id, &caller, &new_key, &nonce) -> client.rotate_payout_key(&program_id, &new_key)
    
    # regex to match: client.rotate_payout_key(arg1, arg2, arg3, arg4)
    # We want to keep arg1 and arg3.
    content = re.sub(
        r'(\w+)\.rotate_payout_key\(([^,]+),\s*[^,]+,\s*([^,]+),\s*[^)]+\)',
        r'\1.rotate_payout_key(\2, \3)',
        content
    )

    # Fix single_payout_v2 calls: 4 args -> 3 args
    # client.single_payout_v2(&program_id, &new_key, &recipient, &1_000) -> client.single_payout_v2(&program_id, &recipient, &1_000)
    content = re.sub(
        r'(\w+)\.single_payout_v2\(([^,]+),\s*[^,]+,\s*([^,]+),\s*([^)]+)\)',
        r'\1.single_payout_v2(\2, \3, \4)',
        content
    )
    
    if os.path.basename(file_path) == 'test_lifecycle.rs':
        # Also need to fix setup_rotation_program call in test_lifecycle.rs if it exists
        # Actually I saw it defined at 1665
        pass

    with open(file_path, 'w') as f:
        f.write(content)
    return True

test_files = ['contracts/program-escrow/src/test_lifecycle.rs']

for f in test_files:
    if os.path.exists(f):
        changed = fix_stale_tests(f)
        print(f"{f}: {'Changed' if changed else 'No change'}")
