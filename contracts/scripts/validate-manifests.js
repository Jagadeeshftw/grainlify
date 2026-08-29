#!/usr/bin/env node

// Contract Manifest Validation Script (Node.js version for cross-platform compatibility)
// This script validates all contract manifests against the schema.

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const Ajv2020 = require('ajv/dist/2020').default;

const colors = {
  red: '\x1b[0;31m',
  green: '\x1b[0;32m',
  yellow: '\x1b[1;33m',
  blue: '\x1b[0;34m',
  nc: '\x1b[0m',
};

function log(color, message) {
  console.log(`${colors[color]}${message}${colors.nc}`);
}

function resolveAjvCommand() {
  const contractsDir = path.dirname(__dirname);

  try {
    execSync('ajv help', { stdio: 'pipe' });
    return 'ajv';
  } catch (e) {
    try {
      execSync(`npx --prefix "${contractsDir}" ajv help`, { stdio: 'pipe' });
      return `npx --prefix "${contractsDir}" ajv`;
    } catch (err) {
      try {
        execSync('npx ajv help', { stdio: 'pipe' });
        return 'npx ajv';
      } catch (nestedErr) {
        return null;
      }
    }
  }
}

function findManifests(dir) {
  let results = [];
  const list = fs.readdirSync(dir);

  list.forEach((file) => {
    const fullPath = path.join(dir, file);
    const stat = fs.statSync(fullPath);
    if (stat && stat.isDirectory() && !fullPath.includes('node_modules')) {
      results = results.concat(findManifests(fullPath));
    } else if (file.endsWith('-manifest.json')) {
      results.push(fullPath);
    }
  });

  return results;
}

function validateManifestFile(manifestPath, schemaPath, ajvCmd = resolveAjvCommand()) {
  if (!ajvCmd) {
    return {
      ok: false,
      errors: ['ajv-cli is not installed. Please install it with: npm install -g ajv-cli'],
    };
  }

  try {
    execSync(`${ajvCmd} validate --spec=draft2020 -c ajv-formats -s "${schemaPath}" -d "${manifestPath}" --verbose`, {
      stdio: 'pipe',
    });
    return { ok: true, errors: [] };
  } catch (error) {
    const stderr = error && error.stderr ? error.stderr.toString() : '';
    const stdout = error && error.stdout ? error.stdout.toString() : '';
    const combined = `${stdout}${stderr}`.trim();
    return {
      ok: false,
      errors: combined ? [combined] : ['Schema validation failed'],
    };
  }
}

function runValidation() {
  const scriptDir = __dirname;
  const contractsDir = path.dirname(scriptDir);
  const schemaPath = path.join(contractsDir, 'contract-manifest-schema.json');
  const ajvCmd = resolveAjvCommand();

  log('blue', '🔍 Contract Manifest Validation');
  log('blue', '==================================');

  if (!ajvCmd) {
    log('red', '❌ ajv-cli is not installed');
    log('nc', 'Please install it with: npm install -g ajv-cli');
    process.exit(1);
  }

  const manifests = findManifests(contractsDir);

  if (manifests.length === 0) {
    log('yellow', '⚠️  No manifest files found');
    process.exit(0);
  }

  let validCount = 0;
  let totalCount = 0;
  const validAuthValues = [
    'admin',
    'signer',
    'any',
    'none',
    'capability',
    'multisig',
    'admin-or-governor',
    'creator',
    'authorized_payout_key',
    'circuit admin',
    'current admin',
    'current controller or admin',
    'existing circuit admin or contract admin',
    'none (first call only)',
    'proposed admin',
    'proposed controller',
  ];

  manifests.forEach((manifest) => {
    totalCount++;
    const manifestName = path.basename(manifest, '.json');

    console.log('');
    log('blue', `📄 Validating ${manifestName}...`);

    const schemaResult = validateManifestFile(manifest, schemaPath, ajvCmd);
    if (!schemaResult.ok) {
      log('red', '❌ Schema validation failed');
      if (schemaResult.errors[0]) {
        console.log(schemaResult.errors[0]);
      }
      return;
    }

    log('green', '✅ Schema validation passed');
    validCount++;

    const manifestData = JSON.parse(fs.readFileSync(manifest, 'utf8'));

    log('blue', '🔍 Checking required fields...');
    const requiredFields = ['contract_name', 'contract_purpose', 'version', 'entrypoints', 'configuration', 'behaviors'];
    let allFieldsPresent = true;

    requiredFields.forEach((field) => {
      if (Object.prototype.hasOwnProperty.call(manifestData, field)) {
        log('green', `  ✅ ${field}`);
      } else {
        log('red', `  ❌ Missing ${field}`);
        allFieldsPresent = false;
      }
    });

    if (!allFieldsPresent) return;

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

    log('blue', '🔍 Checking behaviors structure...');
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

    log('blue', '🔍 Checking authorization values...');
    const authValues = new Set();
    function findAuthValues(obj) {
      if (obj && typeof obj === 'object') {
        if (obj.authorization) {
          authValues.add(obj.authorization);
        }
        Object.values(obj).forEach((value) => findAuthValues(value));
      }
    }

    findAuthValues(manifestData);
    let invalidAuthFound = false;
    authValues.forEach((auth) => {
      if (!validAuthValues.includes(auth)) {
        log('red', `  ❌ Invalid authorization value: ${auth}`);
        invalidAuthFound = true;
      }
    });

    if (!invalidAuthFound) {
      log('green', '  ✅ All authorization values are valid');
    }

    log('blue', '📋 Contract Information:');
    log('green', `  Name: ${manifestData.contract_name}`);
    log('green', `  Purpose: ${manifestData.contract_purpose}`);
    log('green', `  Version: ${currentVersion}`);
    log('green', `  Schema: ${schemaVersion}`);
  });

  console.log('');
  log('blue', '📊 Validation Summary');
  log('blue', '==================================');
  log('blue', `Total manifests: ${totalCount}`);
  log('green', `Valid manifests: ${validCount}`);
  log('red', `Invalid manifests: ${totalCount - validCount}`);

  if (validCount === totalCount) {
    console.log('');
    log('green', '🎉 All manifests are valid!');
    return 0;
  }

  console.log('');
  log('red', '❌ Some manifests have validation errors');
  return 1;
}

module.exports = {
  findManifests,
  validateManifestFile,
  runValidation,
};

if (require.main === module) {
  process.exit(runValidation());
}
