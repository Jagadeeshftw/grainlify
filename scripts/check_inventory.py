import os
import re
import sys

INVENTORY_FILE = 'DEPLOYABLE_ARTIFACTS.md'

def get_cdylib_crates():
    crates = []
    for root, dirs, files in os.walk('.'):
        if 'target' in root or 'node_modules' in root or '.git' in root:
            continue
        for file in files:
            if file == 'Cargo.toml':
                path = os.path.join(root, file)
                with open(path, 'r', encoding='utf-8') as f:
                    content = f.read()
                    if 'crate-type = ["cdylib"]' in content or 'crate-type = ["lib", "cdylib"]' in content or 'crate-type = ["cdylib", "lib"]' in content:
                        # extract name
                        match = re.search(r'name\s*=\s*"([^"]+)"', content)
                        if match:
                            name = match.group(1).replace('-', '_') + '.wasm'
                            crates.append(name)
    return crates

def check_inventory():
    if not os.path.exists(INVENTORY_FILE):
        print(f"Error: {INVENTORY_FILE} not found.")
        sys.exit(1)

    with open(INVENTORY_FILE, 'r', encoding='utf-8') as f:
        inventory_content = f.read()

    crates = get_cdylib_crates()
    missing = []
    for crate in crates:
        # Check if the exact wasm name is in the inventory table
        if f'\{crate}\' not in inventory_content and f'{crate}' not in inventory_content:
            missing.append(crate)

    if missing:
        print(f"Error: The following deployable artifacts are missing from {INVENTORY_FILE}:")
        for m in missing:
            print(f"  - {m}")
        print("Please update the inventory.")
        sys.exit(1)
    
    print("All deployable artifacts are properly listed in the inventory.")

if __name__ == '__main__':
    check_inventory()
