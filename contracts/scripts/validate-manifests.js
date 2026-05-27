#!/usr/bin/env node

// Contract Manifest Validation Script (Node.js version for cross-platform compatibility)
// This script validates all contract manifests against the schema

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const Ajv2020 = require('ajv/dist/2020').default;

// Colors for output
const colors = {
  red: '\x1b[0;31m',
  green: '\x1b[0;32m',
  yellow: '\x1b[1;33m',
  blue: '\x1b[0;34m',
  nc: '\x1b[0m' // No Color
};

function log(color, message) {
  console.log(`${colors[color]}${message}${colors.nc}`);
}

// Script directory
const scriptDir = __dirname;
const contractsDir = path.dirname(scriptDir);

log('blue', '🔍 Contract Manifest Validation');
log('blue', '==================================');

// Check if ajv-cli is installed
try {
  execSync('ajv --version', { stdio: 'pipe' });
} catch (e) {
  // Try with npx as fallback
  try {
    execSync('npx ajv --version', { stdio: 'pipe' });
    process.env.AJV_CMD = 'npx ajv';
  } catch (err) {
    log('red', '❌ ajv-cli is not installed');
    log('nc', 'Please install it with: npm install -g ajv-cli');
    process.exit(1);
  }
}

const ajvCmd = process.env.AJV_CMD || 'ajv';

// Find all manifest files
function findManifests(dir) {
  let results = [];
  const list = fs.readdirSync(dir);
  
  list.forEach(file => {
    file = path.join(dir, file);
    const stat = fs.statSync(file);
    if (stat && stat.isDirectory() && !file.includes('node_modules')) {
      results = results.concat(findManifests(file));
    } else if (file.endsWith('-manifest.json')) {
      results.push(file);
    }
  });
  
  return results;
}

const manifests = findManifests(contractsDir);

if (manifests.length === 0) {
  log('yellow', '⚠️  No manifest files found');
  process.exit(0);
}

let validCount = 0;
let totalCount = 0;

// Valid authorization values
const validAuthValues = ['admin', 'signer', 'any', 'capability', 'multisig'];

// Validate each manifest
manifests.forEach(manifest => {
  totalCount++;
  const manifestName = path.basename(manifest, '.json');
  
  console.log('');
  log('blue', `📄 Validating ${manifestName}...`);
  
  // Validate against schema
  try {
    execSync(`${ajvCmd} validate -s "${path.join(contractsDir, 'contract-manifest-schema.json')}" -d "${manifest}" --verbose`, { 
      stdio: 'ignore' 
    });
    log('green', '✅ Schema validation passed');
    validCount++;
  } catch (e) {
    log('red', '❌ Schema validation failed');
    try {
      execSync(`${ajvCmd} validate -s "${path.join(contractsDir, 'contract-manifest-schema.json')}" -d "${manifest}" --verbose`);
    } catch (err) {
      // Error output already shown
    }
    return; // Continue to next manifest
  }
  
  // Read manifest for further validation
  const manifestData = JSON.parse(fs.readFileSync(manifest, 'utf8'));
  
  // Check required fields
  log('blue', '🔍 Checking required fields...');
  const requiredFields = ['contract_name', 'contract_purpose', 'version', 'entrypoints', 'configuration', 'behaviors'];
  
  let allFieldsPresent = true;
  requiredFields.forEach(field => {
    if (manifestData.hasOwnProperty(field)) {
      log('green', `  ✅ ${field}`);
    } else {
      log('red', `  ❌ Missing ${field}`);
      allFieldsPresent = false;
    }
  });
  
  if (!allFieldsPresent) return;
  
  // Check entrypoints structure
  log('blue', '🔍 Checking entrypoints structure...');
  
  if (manifestData.entrypoints && manifestData.entrypoints.public) {
    log('green', '  ✅ entrypoints.public');
  } else {
    log('red', '  ❌ Missing entrypoints.public');
  }
  
  if (manifestData.entrypoints && manifestData.entrypoints.view) {
    log('green', '  ✅ entrypoints.view');
  } else {
    log('red', '  ❌ Missing entrypoints.view');
  }
  
  // Check behaviors structure
  log('blue', '🔍 Checking behaviors structure...');
  // Initialize AJV
  const ajv = new Ajv2020({
    allErrors: true,
    strict: false,
    validateFormats: false,
  });
  
  if (manifestData.behaviors && manifestData.behaviors.security_features) {
    log('green', '  ✅ behaviors.security_features');
  } else {
    log('red', '  ❌ Missing behaviors.security_features');
  }
  
  if (manifestData.behaviors && manifestData.behaviors.access_control) {
    log('green', '  ✅ behaviors.access_control');
  } else {
    log('red', '  ❌ Missing behaviors.access_control');
  }
  
  // Validate version format
  log('blue', '🔍 Checking version format...');
  
  const currentVersion = manifestData.version.current;
  const schemaVersion = manifestData.version.schema;
  
  const versionRegex = /^[0-9]+\.[0-9]+\.[0-9]+$/;
  
  if (versionRegex.test(currentVersion)) {
    log('green', `  ✅ Current version format: ${currentVersion}`);
  } else {
    log('red', `  ❌ Invalid current version format: ${currentVersion}`);
  }
  
  if (versionRegex.test(schemaVersion)) {
    log('green', `  ✅ Schema version format: ${schemaVersion}`);
  } else {
    log('red', `  ❌ Invalid schema version format: ${schemaVersion}`);
  }
  
  // Validate authorization values
  log('blue', '🔍 Checking authorization values...');
  
  function findAuthValues(obj, authValues = new Set()) {
    if (obj && typeof obj === 'object') {
      if (obj.authorization) {
        authValues.add(obj.authorization);
      }
      Object.values(obj).forEach(value => findAuthValues(value, authValues));
    }
    return authValues;
  }
  
  const authValues = findAuthValues(manifestData);
  let invalidAuthFound = false;
  
  authValues.forEach(auth => {
    if (!validAuthValues.includes(auth)) {
      log('red', `  ❌ Invalid authorization value: ${auth}`);
      invalidAuthFound = true;
    }
  });
  
  if (!invalidAuthFound) {
    log('green', '  ✅ All authorization values are valid');
  }
  
  // Display contract info
  log('blue', '📋 Contract Information:');
  log('green', `  Name: ${manifestData.contract_name}`);
  log('green', `  Purpose: ${manifestData.contract_purpose}`);
  log('green', `  Version: ${currentVersion}`);
  log('green', `  Schema: ${schemaVersion}`);
});

// Summary
console.log('');
log('blue', '📊 Validation Summary');
log('blue', '==================================');
log('blue', `Total manifests: ${totalCount}`);
log('green', `Valid manifests: ${validCount}`);
log('red', `Invalid manifests: ${totalCount - validCount}`);

if (validCount === totalCount) {
  console.log('');
  log('green', '🎉 All manifests are valid!');
  process.exit(0);
} else {
  console.log('');
  log('red', '❌ Some manifests have validation errors');
  process.exit(1);
}
