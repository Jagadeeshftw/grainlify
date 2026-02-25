#!/usr/bin/env node

/**
 * Node.js version of the manifest validation script
 * This provides cross-platform compatibility
 */

const fs = require('fs');
const path = require('path');
const Ajv = require('ajv');

// Configuration
const CONTRACTS_DIR = path.join(__dirname, '..');
const SCHEMA_FILE = path.join(CONTRACTS_DIR, 'contract-manifest-schema.json');

// Colors for console output
const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m'
};

function colorLog(color, message) {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

/**
 * Load and validate the schema
 */
function loadSchema() {
  try {
    const schemaContent = fs.readFileSync(SCHEMA_FILE, 'utf8');
    return JSON.parse(schemaContent);
  } catch (error) {
    colorLog('red', `❌ Error loading schema: ${error.message}`);
    process.exit(1);
  }
}

/**
 * Find all manifest files
 */
function findManifests() {
  const files = fs.readdirSync(CONTRACTS_DIR);
  return files
    .filter(file => file.endsWith('-manifest.json'))
    .map(file => path.join(CONTRACTS_DIR, file));
}

/**
 * Validate a single manifest
 */
function validateManifest(ajv, schema, manifestPath) {
  const manifestName = path.basename(manifestPath, '.json');
  
  try {
    const manifestContent = fs.readFileSync(manifestPath, 'utf8');
    const manifest = JSON.parse(manifestContent);
    
    // Validate against schema
    const validate = ajv.compile(schema);
    const isValid = validate(manifest);
    
    if (!isValid) {
      colorLog('red', `❌ ${manifestName} validation failed:`);
      validate.errors.forEach(error => {
        colorLog('red', `   - ${error.instancePath || 'root'}: ${error.message}`);
      });
      return false;
    }
    
    // Additional structural checks
    const errors = [];
    
    // Check required top-level fields
    const requiredFields = ['contract_name', 'contract_purpose', 'version', 'entrypoints', 'configuration', 'behaviors'];
    requiredFields.forEach(field => {
      if (!manifest[field]) {
        errors.push(`Missing required field: ${field}`);
      }
    });
    
    // Check entrypoints structure
    if (manifest.entrypoints) {
      if (!manifest.entrypoints.public) {
        errors.push('Missing entrypoints.public');
      }
      if (!manifest.entrypoints.view) {
        errors.push('Missing entrypoints.view');
      }
    }
    
    // Check behaviors structure
    if (manifest.behaviors) {
      if (!manifest.behaviors.security_features) {
        errors.push('Missing behaviors.security_features');
      }
      if (!manifest.behaviors.access_control) {
        errors.push('Missing behaviors.access_control');
      }
    }
    
    // Validate version format
    if (manifest.version) {
      const currentVersion = manifest.version.current;
      const schemaVersion = manifest.version.schema;
      
      if (currentVersion && !/^\d+\.\d+\.\d+$/.test(currentVersion)) {
        errors.push(`Invalid current version format: ${currentVersion} (should be x.y.z)`);
      }
      
      if (schemaVersion && !/^\d+\.\d+\.\d+$/.test(schemaVersion)) {
        errors.push(`Invalid schema version format: ${schemaVersion} (should be x.y.z)`);
      }
    }
    
    // Validate authorization values
    const validAuthValues = ['admin', 'signer', 'any', 'capability', 'multisig'];
    const authValues = [];
    
    function extractAuthValues(obj, path = '') {
      if (typeof obj === 'object' && obj !== null) {
        if (obj.authorization) {
          authValues.push({ value: obj.authorization, path });
        }
        Object.values(obj).forEach(value => extractAuthValues(value, path));
      }
    }
    
    extractAuthValues(manifest);
    
    authValues.forEach(({ value, path }) => {
      if (!validAuthValues.includes(value)) {
        errors.push(`Invalid authorization value "${value}" at ${path}`);
      }
    });
    
    if (errors.length > 0) {
      colorLog('red', `❌ ${manifestName} structural validation failed:`);
      errors.forEach(error => {
        colorLog('red', `   - ${error}`);
      });
      return false;
    }
    
    colorLog('green', `✅ ${manifestName} is valid`);
    
    // Display contract info
    colorLog('blue', `   Contract: ${manifest.contract_name}`);
    colorLog('blue', `   Version: ${manifest.version.current}`);
    colorLog('blue', `   Purpose: ${manifest.contract_purpose.substring(0, 80)}${manifest.contract_purpose.length > 80 ? '...' : ''}`);
    
    return true;
    
  } catch (error) {
    colorLog('red', `❌ Error processing ${manifestName}: ${error.message}`);
    return false;
  }
}

/**
 * Main validation function
 */
function main() {
  colorLog('cyan', '🔍 Contract Manifest Validation');
  console.log('==================================');
  
  // Load schema
  const schema = loadSchema();
  
  // Initialize AJV
  const ajv = new Ajv({ allErrors: true });
  
  // Find manifests
  const manifests = findManifests();
  
  if (manifests.length === 0) {
    colorLog('yellow', '⚠️  No manifest files found');
    process.exit(0);
  }
  
  colorLog('blue', `📋 Found ${manifests.length} manifest(s)`);
  
  // Validate each manifest
  let validCount = 0;
  manifests.forEach(manifestPath => {
    if (validateManifest(ajv, schema, manifestPath)) {
      validCount++;
    }
    console.log('');
  });
  
  // Summary
  colorLog('cyan', '📊 Validation Summary');
  console.log('==================================');
  colorLog('blue', `Total manifests: ${manifests.length}`);
  colorLog('green', `Valid manifests: ${validCount}`);
  colorLog('red', `Invalid manifests: ${manifests.length - validCount}`);
  
  if (validCount === manifests.length) {
    colorLog('green', '🎉 All manifests are valid!');
    process.exit(0);
  } else {
    colorLog('red', '❌ Some manifests have validation errors');
    process.exit(1);
  }
}

// Run if called directly
if (require.main === module) {
  main();
}

module.exports = {
  validateManifest,
  loadSchema,
  findManifests
};
