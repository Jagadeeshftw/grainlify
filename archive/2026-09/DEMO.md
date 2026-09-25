# Contract Upgrade Demo

This document demonstrates the working contract upgrade system.

![Demo Screenshot](assets/demo_screenshot.png)

## Demo Script
A script `scripts/demo_upgrade.sh` has been created to automate the following steps:
1.  Build Version 1 of the contract.
2.  Deploy V1 to Testnet.
3.  Initialize V1.
4.  Verify Version is 1.
5.  Modify source code to Version 2 (changing `get_version` to return 2).
6.  Build V2.
7.  Upgrade the deployed contract to V2 code.
8.  Verify Version is 2.

## Execution Log
Below is the output of running the demo script:

```
=== Grainlify Contract Upgrade Demo ===
[1/9] Building V1...
   Compiling grainlify-core v0.1.0 (/home/knights/Documents/Project/Drips/grainlify/contracts/grainlify-core)
    Finished `release` profile [optimized] target(s) in 0.39s
[2/9] Setting up Identity...
[3/9] Deploying V1...
Contract Deployed: CA... (ID hidden for brevity)
[4/9] Initializing V1...
ℹ️  Signing transaction: ...
[5/9] Checking Version (Expect: 1)...
Current Version: 1
[6/9] Modifying code to Version 2...
[7/9] Building V2...
   Compiling grainlify-core v0.1.0 (/home/knights/Documents/Project/Drips/grainlify/contracts/grainlify-core)
    Finished `release` profile [optimized] target(s) in 0.39s
[8/9] Upgrading Contract...
Uploading WASM...
ℹ️  Simulating install transaction…
ℹ️  Signing transaction: ...
🌎 Submitting install transaction…
WASM Hash: c73e471d9e5eaa118b6b50b01e41503c19f433d932ac3848049291e1d8c732d2
Upgrading contract...
ℹ️  Signing transaction: ...
Upgrade complete.
[9/9] Checking Version (Expect: 2)...
Current Version: 2
=== Demo Successful ===
Restoring source code...
```

## How to Run
You can run this demo yourself:
```bash
./scripts/demo_upgrade.sh
```
*Note: Requires `soroban-cli` installed and configured.*
