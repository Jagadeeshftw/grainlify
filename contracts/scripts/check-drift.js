#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Colors for output
const colors = {
  red: '\x1b[0;31m',
  green: '\x1b[0;32m',
  yellow: '\x1b[1;33m',
  blue: '\x1b[0;34m',
  nc: '\x1b[0m'
};

function log(color, message) {
  console.log(`${colors[color]}${message}${colors.nc}`);
}

const scriptDir = __dirname;
const contractsDir = path.dirname(scriptDir);
const configPath = path.join(scriptDir, 'drift-check-config.json');

// Load config
let config = { contracts: {} };
if (fs.existsSync(configPath)) {
  try {
    config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    log('green', '✅ Drift check config loaded successfully');
  } catch (e) {
    log('red', '⚠️  Failed to load config: ' + e.message);
  }
}

const targets = [
  {
    name: 'grainlify-core',
    contractStruct: 'GrainlifyContract',
    manifest: path.join(contractsDir, 'grainlify-core-manifest.json'),
    srcDir: path.join(contractsDir, 'grainlify-core', 'src')
  },
  {
    name: 'bounty-escrow',
    contractStruct: 'BountyEscrowContract',
    manifest: path.join(contractsDir, 'bounty-escrow-manifest.json'),
    srcDir: path.join(contractsDir, 'bounty_escrow', 'contracts', 'escrow', 'src')
  },
  {
    name: 'program-escrow',
    contractStruct: 'ProgramEscrowContract',
    manifest: path.join(contractsDir, 'program-escrow-manifest.json'),
    srcDir: path.join(contractsDir, 'program-escrow', 'src')
  }
];

// Helper to recursively find all .rs files
function findRustFiles(dir) {
  let results = [];
  if (!fs.existsSync(dir)) return results;
  const list = fs.readdirSync(dir);
  list.forEach(file => {
    const fullPath = path.join(dir, file);
    const stat = fs.statSync(fullPath);
    if (stat && stat.isDirectory()) {
      // Exclude tests folders
      if (file !== 'tests' && file !== 'test') {
        results = results.concat(findRustFiles(fullPath));
      }
    } else if (file.endsWith('.rs')) {
      // Exclude test files
      if (!file.startsWith('test_') && !file.endsWith('_tests.rs') && file !== 'test.rs' && !file.includes('test_')) {
        results.push(fullPath);
      }
    }
  });
  return results;
}

// Extract public entrypoints from Rust content
function extractEntrypoints(content, contractStruct) {
  const entrypoints = new Set();
  
  // Find contractimpl blocks for the specific contract struct
  const implBlockRegex = new RegExp('#\\[contractimpl\\]\\s*(?:impl\\s+(?:[\\w:]+\\s+for\\s+)?' + contractStruct + '\\b\\s*\\{)', 'g');
  let match;
  while ((match = implBlockRegex.exec(content)) !== null) {
    const startIdx = match.index + match[0].length - 1; // position of '{'
    
    // Match braces to find the end of impl block
    let braceCount = 1;
    let i = startIdx + 1;
    while (braceCount > 0 && i < content.length) {
      if (content[i] === '{') braceCount++;
      else if (content[i] === '}') braceCount--;
      i++;
    }
    
    const implBody = content.substring(startIdx, i);
    
    // Extract pub fn
    const fnRegex = /pub\s+fn\s+(\w+)\s*\(/g;
    let fnMatch;
    while ((fnMatch = fnRegex.exec(implBody)) !== null) {
      entrypoints.add(fnMatch[1]);
    }
  }
  
  return Array.from(entrypoints);
}

// Extract DataKey variants from Rust content
function extractDataKeys(content) {
  const keys = new Set();
  const enumRegex = /pub\s+enum\s+(DataKey|HookDataKey)\s*\{([\s\S]*?)\}/g;
  let match;
  while ((match = enumRegex.exec(content)) !== null) {
    const body = match[2];
    
    // Split by comma/newlines and parse variants
    const lines = body.split('\n');
    lines.forEach(line => {
      // Clean comments and whitespace
      const cleanLine = line.replace(/\/\/.*/, '').trim();
      if (!cleanLine) return;
      
      // Match variants like "Admin" or "UpgradeProposal(u64)"
      const variantMatch = /^(\w+)(?:\s*\([^)]*\))?\s*,?/.exec(cleanLine);
      if (variantMatch) {
        const name = variantMatch[1];
        if (name && name !== 'pub' && name !== 'enum') {
          // Extract the full variant name to match manifest layout
          const fullVariant = cleanLine.replace(/,$/, '').replace(/=.*$/, '').trim();
          if (fullVariant) {
            keys.add(fullVariant);
          }
        }
      }
    });
  }
  return Array.from(keys);
}

// Extract Event structs from Rust content
function extractEvents(content) {
  const events = new Set();
  
  // Find all pub struct with Event suffix or contracttype attribute
  const structRegex = /pub\s+struct\s+(\w+Event)\b/g;
  let match;
  while ((match = structRegex.exec(content)) !== null) {
    events.add(match[1]);
  }
  
  return Array.from(events);
}

let driftFound = false;

targets.forEach(target => {
  console.log('\n==================================================');
  log('blue', `🔍 Analyzing drift for: ${target.name}`);
  console.log('==================================================');
  
  if (!fs.existsSync(target.manifest)) {
    log('red', `❌ Manifest file not found: ${target.manifest}`);
    driftFound = true;
    return;
  }
  
  let manifestData;
  try {
    manifestData = JSON.parse(fs.readFileSync(target.manifest, 'utf8'));
  } catch (e) {
    log('red', `❌ Failed to parse manifest: ${e.message}`);
    driftFound = true;
    return;
  }
  
  // Load target config
  const targetConfig = config.contracts[target.name] || { aliases: {}, exemptions: [] };
  const aliases = targetConfig.aliases || {};
  const exemptions = new Set(targetConfig.exemptions || []);
  
  // Read all rust files
  const files = findRustFiles(target.srcDir);
  let rustEntrypoints = [];
  let rustDataKeys = [];
  let rustEvents = [];
  
  files.forEach(file => {
    const content = fs.readFileSync(file, 'utf8');
    rustEntrypoints = rustEntrypoints.concat(extractEntrypoints(content, target.contractStruct));
    rustDataKeys = rustDataKeys.concat(extractDataKeys(content));
    rustEvents = rustEvents.concat(extractEvents(content));
  });
  
  // Remove duplicates
  rustEntrypoints = Array.from(new Set(rustEntrypoints));
  rustDataKeys = Array.from(new Set(rustDataKeys));
  rustEvents = Array.from(new Set(rustEvents));
  
  // Extract manifest entrypoints
  const manifestEntrypoints = [];
  if (manifestData.entrypoints) {
    ['public', 'admin', 'view'].forEach(type => {
      if (manifestData.entrypoints[type]) {
        manifestData.entrypoints[type].forEach(ep => {
          manifestEntrypoints.push(ep.name);
        });
      }
    });
  }
  
  // Extract manifest storage keys
  const manifestDataKeys = [];
  if (manifestData.configuration && manifestData.configuration.storage_keys) {
    manifestData.configuration.storage_keys.forEach(key => {
      manifestDataKeys.push(key.name);
    });
  }
  
  // Extract manifest events
  const manifestEvents = [];
  if (manifestData.events) {
    manifestData.events.forEach(ev => {
      manifestEvents.push(ev.name);
    });
  }
  
  // Helper to check match using aliases and exemptions
  function isMatched(item, manifestList, type) {
    if (exemptions.has(item)) return true;
    
    // Check direct match
    if (manifestList.includes(item)) return true;
    
    // Check alias
    const alias = aliases[item];
    if (alias && manifestList.includes(alias)) return true;
    
    return false;
  }
  
  function isManifestMatched(manifestItem, rustList, type) {
    if (exemptions.has(manifestItem)) return true;
    
    // Check direct match
    if (rustList.includes(manifestItem)) return true;
    
    // Check alias in reverse
    for (const [rustName, aliasName] of Object.entries(aliases)) {
      if (aliasName === manifestItem && rustList.includes(rustName)) {
        return true;
      }
    }
    
    return false;
  }
  
  // 1. Check entrypoints drift
  log('blue', 'Checking public entrypoints...');
  let entrypointDrift = false;
  
  rustEntrypoints.forEach(ep => {
    if (!isMatched(ep, manifestEntrypoints, 'entrypoint')) {
      log('yellow', `  ⚠️  Undocumented entrypoint in Rust: ${ep}`);
      entrypointDrift = true;
    }
  });
  
  manifestEntrypoints.forEach(ep => {
    if (!isManifestMatched(ep, rustEntrypoints, 'entrypoint')) {
      log('yellow', `  ⚠️  Stale entrypoint in manifest (removed from Rust): ${ep}`);
      entrypointDrift = true;
    }
  });
  
  if (!entrypointDrift) {
    log('green', '  ✅ Entrypoints are fully in sync');
  } else {
    driftFound = true;
  }
  
  // 2. Check storage keys drift
  log('blue', 'Checking storage keys...');
  let storageDrift = false;
  
  rustDataKeys.forEach(key => {
    if (!isMatched(key, manifestDataKeys, 'storage_key')) {
      log('yellow', `  ⚠️  Undocumented storage key in Rust: ${key}`);
      storageDrift = true;
    }
  });
  
  manifestDataKeys.forEach(key => {
    if (!isManifestMatched(key, rustDataKeys, 'storage_key')) {
      log('yellow', `  ⚠️  Stale storage key in manifest (removed from Rust): ${key}`);
      storageDrift = true;
    }
  });
  
  if (!storageDrift) {
    log('green', '  ✅ Storage keys are fully in sync');
  } else {
    driftFound = true;
  }
  
  // 3. Check events drift
  log('blue', 'Checking events...');
  let eventDrift = false;
  
  rustEvents.forEach(ev => {
    if (!isMatched(ev, manifestEvents, 'event')) {
      log('yellow', `  ⚠️  Undocumented event in Rust: ${ev}`);
      eventDrift = true;
    }
  });
  
  manifestEvents.forEach(ev => {
    if (!isManifestMatched(ev, rustEvents, 'event')) {
      log('yellow', `  ⚠️  Stale event in manifest (removed from Rust): ${ev}`);
      eventDrift = true;
    }
  });
  
  if (!eventDrift) {
    log('green', '  ✅ Events are fully in sync');
  } else {
    driftFound = true;
  }
});

console.log('');
log('blue', '📊 Drift Check Summary');
console.log('==================================');
if (driftFound) {
  log('red', '❌ Drift detected between Rust code and manifests!');
  log('red', 'Please document the undocumented items in manifests, or add them as exemptions/aliases in contracts/scripts/drift-check-config.json.');
  process.exit(1);
} else {
  log('green', '🎉 All documentation is in sync. No drift detected!');
  process.exit(0);
}
