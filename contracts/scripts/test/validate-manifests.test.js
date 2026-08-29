const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const {
  loadValidator,
  validateManifest,
} = require('../validate-manifests');

const fixturesDir = path.join(__dirname, 'fixtures');
const fixture = name => path.join(fixturesDir, name);
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));

const validate = loadValidator();

test('accepts a minimal manifest fixture', () => {
  const result = validateManifest(fixture('valid-minimal.json'), validate);
  assert.deepEqual(result, { valid: true, errors: [] });
});

test('accepts every production manifest', () => {
  for (const name of ['bounty-escrow-manifest.json', 'grainlify-core-manifest.json', 'program-escrow-manifest.json']) {
    const result = validateManifest(path.join(__dirname, '..', '..', name), validate);
    assert.equal(result.valid, true, `${name}: ${result.errors.join('; ')}`);
  }
});

test('rejects missing top-level fields', () => {
  const result = validateManifest(fixture('missing-fields.json'), validate);
  assert.equal(result.valid, false);
  assert.match(result.errors.join('\n'), /required property 'contract_name'/);
});

test('covers every required-field rule', () => {
  const requiredFields = [
    ['contract_name', []],
    ['contract_purpose', []],
    ['version', []],
    ['entrypoints', []],
    ['configuration', []],
    ['behaviors', []],
    ['current', ['version']],
    ['schema', ['version']],
    ['public', ['entrypoints']],
    ['view', ['entrypoints']],
    ['parameters', ['configuration']],
    ['security_features', ['behaviors']],
    ['access_control', ['behaviors']],
  ];

  for (const [field, parents] of requiredFields) {
    const data = readJson(fixture('valid-minimal.json'));
    let target = data;
    for (const parent of parents) target = target[parent];
    delete target[field];

    assert.equal(validate(data), false, `expected missing ${parents.concat(field).join('.')}`);
    const errors = validate.errors.map(error => `${error.instancePath}/${error.params.missingProperty}`);
    const expectedPath = `/${parents.join('/')}${parents.length ? '/' : ''}${field}`;
    assert.ok(
      errors.includes(expectedPath),
      `missing rule not reported for ${parents.concat(field).join('.')}`,
    );
  }
});

test('rejects unsupported authorization with an actionable path and values', () => {
  const result = validateManifest(fixture('unsupported-authorization.json'), validate);
  assert.equal(result.valid, false);
  assert.match(result.errors.join('\n'), /\/entrypoints\/public\/0\/authorization/);
  assert.match(result.errors.join('\n'), /Allowed values/);
});

test('rejects invalid semantic versions with field paths', () => {
  const result = validateManifest(fixture('invalid-semver.json'), validate);
  assert.equal(result.valid, false);
  assert.match(result.errors.join('\n'), /\/version\/current/);
  assert.match(result.errors.join('\n'), /\/version\/schema/);
  assert.match(result.errors.join('\n'), /pattern/);
});

test('rejects a manifest missing the public or view entrypoint sections', () => {
  const result = validateManifest(fixture('missing-view-public.json'), validate);
  assert.equal(result.valid, false);
  assert.match(result.errors.join('\n'), /\/entrypoints: required property 'view'/);

  const data = readJson(fixture('valid-minimal.json'));
  delete data.entrypoints.public;
  assert.equal(validate(data), false);
  assert.ok(validate.errors.some(error => error.params.missingProperty === 'public'));
});
