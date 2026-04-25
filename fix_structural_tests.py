import os
import re

def fix_structural_tests(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Fix ProgramData initialization: remove schema_version and wrap metadata in Some()
    # Pattern: ProgramData { ... metadata: crate::ProgramMetadata::empty(&env), ... schema_version: ... }
    
    # 1. Wrap metadata in Some()
    content = content.replace('metadata: crate::ProgramMetadata::empty(&env)', 'metadata: Some(crate::ProgramMetadata::empty(&env))')
    content = content.replace('metadata: ProgramMetadata::empty(&env)', 'metadata: Some(ProgramMetadata::empty(&env))')
    
    # 2. Remove schema_version: STORAGE_SCHEMA_VERSION,
    content = re.sub(r'\s*schema_version:\s*STORAGE_SCHEMA_VERSION,?', '', content)
    
    # 3. Add status: ProgramStatus::Active (or similar) if it's missing in ProgramData initialization
    # If ProgramData initialization is found, check if status is there.
    # ProgramData { ... }
    
    def add_status(match):
        fields = match.group(1)
        if 'status:' not in fields:
            # Add status field
            if fields.strip().endswith(','):
                return f'ProgramData {{{fields} status: ProgramStatus::Active, }}'
            else:
                return f'ProgramData {{{fields}, status: ProgramStatus::Active, }}'
        return match.group(0)

    content = re.sub(r'ProgramData\s*\{([^}]+)\}', add_status, content)

    with open(file_path, 'w') as f:
        f.write(content)
    return True

test_files = [
    'contracts/program-escrow/src/test_serialization_compatibility.rs',
    'contracts/program-escrow/src/test_payout_splits.rs'
]

for f in test_files:
    if os.path.exists(f):
        changed = fix_structural_tests(f)
        print(f"{f}: {'Changed' if changed else 'No change'}")
