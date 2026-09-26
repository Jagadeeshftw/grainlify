#!/usr/bin/env node
const Ajv = require('ajv/dist/2020');
const addFormats = require('ajv-formats');
const fs = require('fs');
const path = require('path');

const scriptDir = __dirname;
const contractsDir = path.dirname(scriptDir);
const projectRoot = path.join(contractsDir, '..');
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
  if (!fs.existsSync(dir)) return results;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory() && !['node_modules', 'target', 'target-final', 'target-fresh', 'target-gov', 'target-t2', '.git', '.github', 'deployments'].includes(entry.name)) {
      results.push(...findManifests(entryPath));
    } else if (entry.isFile() && entry.name.endsWith('-manifest.json') && entry.name !== 'storage-layout-manifest.json') {
      results.push(entryPath);
    }
  }
  return results;
}

function findDeployableCrates(dir) {
  const crates = [];
  if (!fs.existsSync(dir)) return crates;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory() && !['node_modules', 'target', 'target-final', 'target-fresh', 'target-gov', 'target-t2', '.git', '.github', 'deployments'].includes(entry.name)) {
      crates.push(...findDeployableCrates(entryPath));
    } else if (entry.isFile() && entry.name === 'Cargo.toml') {
      const content = fs.readFileSync(entryPath, 'utf8');
      const hasCdylib = content.includes('crate-type') && content.includes('"cdylib"');
      const isSorobanContract = entryPath.replace(/\\/g, '/').includes('soroban/contracts/');
      if (hasCdylib || isSorobanContract) {
        crates.push(dir);
      }
    }
  }
  return crates;
}

function getCrateName(cargoTomlPath) {
  const content = fs.readFileSync(cargoTomlPath, 'utf8');
  const match = content.match(/name\s*=\s*"([^"]+)"/);
  return match ? match[1] : null;
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

function run() {
  const manifestPaths = findManifests(projectRoot);
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

  // Ensure every deployable crate has a manifest.
  const deployableDirs = findDeployableCrates(projectRoot);
  let missing = 0;
  for (const dir of deployableDirs) {
    const crateName = getCrateName(path.join(dir, 'Cargo.toml'));
    if (!crateName) continue;
    const hasManifest = manifestPaths.some(m => {
      const basename = path.basename(m);
      return path.dirname(m) === dir || basename.includes(crateName) || basename.includes(crateName.replace(/_/g, '-'));
    });
    if (!hasManifest) {
      log('red', `Deployable crate '${crateName}' at ${dir} is missing a manifest.`);
      missing++;
    }
  }

  console.log('');
  log('blue', `Total manifests: ${manifestPaths.length}`);
  log('green', `Valid manifests: ${validCount}`);
  if (invalidCount > 0) {
    log('red', `Invalid manifests: ${invalidCount}`);
    return 1;
  }

  if (missing > 0) {
    log('red', `Missing manifests for ${missing} deployable crate(s).`);
    return 1;
  }

  log('green', 'All manifests are valid and present for all deployable crates.');
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
