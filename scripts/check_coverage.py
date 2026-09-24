import os
import re
import sys

LIB_RS = "contracts/bounty_escrow/contracts/escrow/src/lib.rs"
SRC_DIR = "contracts/bounty_escrow/contracts/escrow/src"
MAP_FILE = "contracts/bounty_escrow/COVERAGE_MAP.md"

def main():
    if not os.path.exists(LIB_RS):
        print(f"Error: {LIB_RS} not found.")
        sys.exit(1)

    with open(LIB_RS, "r", encoding="utf-8") as f:
        content = f.read()
    
    pub_fns = re.findall(r'pub fn ([a-zA-Z0-9_]+)', content)
    unique_pub_fns = list(set(pub_fns))
    unique_pub_fns.sort()

    test_files = []
    for root, dirs, files in os.walk(SRC_DIR):
        for file in files:
            if (file.startswith('test') and file.endswith('.rs')) or (file == "test.rs"):
                test_files.append(os.path.join(root, file))

    mapping = {fn: set() for fn in unique_pub_fns}
    
    for test_file in test_files:
        with open(test_file, "r", encoding="utf-8") as f:
            test_content = f.read()
            
            test_blocks = re.split(r'fn\s+test_', test_content)
            for i, block in enumerate(test_blocks):
                if i == 0: continue
                match = re.match(r'([a-zA-Z0-9_]+)', block)
                if not match: continue
                test_name = "test_" + match.group(1)
                
                for fn in unique_pub_fns:
                    if re.search(r'\b' + fn + r'\s*\(', block):
                        mapping[fn].add(test_name)
    
    # Check if MAP_FILE exists to preserve manual justifications
    existing_justifications = {}
    if os.path.exists(MAP_FILE):
        with open(MAP_FILE, "r", encoding="utf-8") as f:
            current_fn = None
            for line in f:
                fn_match = re.match(r'^## `([a-zA-Z0-9_]+)`', line)
                if fn_match:
                    current_fn = fn_match.group(1)
                elif current_fn:
                    just_match = re.search(r'\- \*\*UNCOVERED\*\*\s*\(Justification:\s*(.*?)\)', line)
                    if just_match:
                        existing_justifications[current_fn] = just_match.group(1)

    unmapped = []
    
    with open(MAP_FILE, "w", encoding="utf-8") as f:
        f.write("# Entry Point Coverage Map\n\n")
        f.write("This file is automatically verified by CI. Any `pub fn` in `lib.rs` must either be called by a test, or manually justified here.\n\n")
        
        for fn in unique_pub_fns:
            f.write(f"## `{fn}`\n")
            if mapping[fn]:
                for t in sorted(mapping[fn]):
                    f.write(f"- `{t}`\n")
            else:
                if fn in existing_justifications:
                    f.write(f"- **UNCOVERED** (Justification: {existing_justifications[fn]})\n")
                else:
                    # Provide a generic justification which the developer should update
                    f.write("- **UNCOVERED** (Justification: Pending manual review)\n")
                    unmapped.append(fn)
            f.write("\n")

    if unmapped:
        print(f"FAILED: Found {len(unmapped)} entry points with no tests and no justification in {MAP_FILE}.")
        print("Unmapped entry points:")
        for fn in unmapped:
            print(f" - {fn}")
        print("\nPlease add tests or update COVERAGE_MAP.md with a justification.")
        sys.exit(1)
    
    print("SUCCESS: All entry points are mapped or justified.")
    sys.exit(0)

if __name__ == "__main__":
    main()
