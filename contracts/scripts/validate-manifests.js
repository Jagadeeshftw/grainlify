#!/usr/bin/env node

// Contract manifest validation. The JSON Schema is the source of truth for
// required fields, entrypoint sections, versions, and authorization values.

const Ajv = require('ajv/dist/2020');
const addFormats = require('ajv-formats');
const fs = require('fs');
const path = require('path');

const scriptDir = __dirname;
const contractsDir = path.dirname(scriptDir);
const schemaPath = path.join(contractsDir, 'contract-manifest-schema.json');

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

function formatError(error) {
  const location = error.instancePath || '/';
  const rule = error.keyword === 'required'
    ? `required property '${error.params.missingProperty}'`
    : error.keyword;
  const expected = error.keyword === 'enum'
    ? ` Allowed values: ${error.params.allowedValues.join(', ')}.`
    : '';
  return `${location}: ${rule} - ${error.message}.${expected}`;
}

function loadValidator() {
  const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
  const ajv = new Ajv({ allErrors: true, strict: false });
  addFormats(ajv);
  return ajv.compile(schema);
}

function findManifests(dir) {
  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'node_modules') {
      results.push(...findManifests(entryPath));
    } else if (entry.isFile() && entry.name.endsWith('-manifest.json')) {
      results.push(entryPath);
    }
  }
  return results;
}

function validateManifest(manifestPath, validate) {
  let data;
  try {
    data = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  } catch (error) {
    return { valid: false, errors: [`/: parse error - ${error.message}`] };
  }

  if (validate(data)) {
    return { valid: true, errors: [] };
  }

  return {
    valid: false,
    errors: (validate.errors || []).map(formatError),
  };
}

function run(manifestPaths = findManifests(contractsDir)) {
  let validCount = 0;

  if (manifestPaths.length === 0) {
    log('yellow', 'No manifest files found');
    return 0;
  }

  let validate;
  try {
    validate = loadValidator();
  } catch (error) {
    log('red', `Failed to load manifest schema: ${error.message}`);
    return 1;
  }

  log('blue', 'Contract Manifest Validation');
  log('blue', '=============================');

  for (const manifestPath of manifestPaths) {
    const result = validateManifest(manifestPath, validate);
    const displayPath = path.relative(process.cwd(), manifestPath);
    console.log('');
    log('blue', `Validating ${displayPath}...`);
    if (result.valid) {
      log('green', 'Schema validation passed');
      validCount += 1;
    } else {
      log('red', 'Schema validation failed');
      for (const error of result.errors) {
        log('red', `  ${error}`);
      }
    }
  }

  const invalidCount = manifestPaths.length - validCount;
  console.log('');
  log('blue', `Total manifests: ${manifestPaths.length}`);
  log('green', `Valid manifests: ${validCount}`);
  if (invalidCount > 0) {
    log('red', `Invalid manifests: ${invalidCount}`);
    return 1;
  }
  log('green', 'All manifests are valid');
  return 0;
}

if (require.main === module) {
  process.exitCode = run();
}

module.exports = {
  findManifests,
  formatError,
  loadValidator,
  run,
  validateManifest,
};
