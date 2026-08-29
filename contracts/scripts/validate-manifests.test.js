const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const repoRoot = path.resolve(__dirname, '..', '..');
const rootWorkflowPath = path.join(repoRoot, '.github', 'workflows', 'validate-manifests.yml');
const legacyWorkflowPath = path.join(repoRoot, 'contracts', '.github', 'workflows', 'validate-manifests.yml');

const manifestValidator = require('./validate-manifests.js');

test('manifest workflow is stored in the repository root workflow directory', () => {
  assert.ok(fs.existsSync(rootWorkflowPath), 'expected root workflow to exist');
  assert.ok(!fs.existsSync(legacyWorkflowPath), 'legacy nested workflow should be removed to avoid duplicate runs');

  const workflowContents = fs.readFileSync(rootWorkflowPath, 'utf8');
  assert.match(workflowContents, /pull_request\s*:/, 'workflow should trigger on PRs');
  assert.match(workflowContents, /contracts\/.*-manifest\.json/, 'workflow should match manifest files');
  assert.match(workflowContents, /authorized_payout_key/, 'workflow should allow the payout-key authorization value required by manifests');
  assert.match(workflowContents, /contract-manifest-schema\.json/, 'workflow should keep schema validation');
});

test('validate-manifests rejects malformed manifest files', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'manifest-validator-'));
  const schemaPath = path.join(tempDir, 'contract-manifest-schema.json');
  const invalidManifestPath = path.join(tempDir, 'broken-manifest.json');

  fs.writeFileSync(schemaPath, JSON.stringify({
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    type: 'object',
    properties: {
      contract_name: { type: 'string' },
      contract_purpose: { type: 'string' },
      version: { type: 'object' }
    },
    required: ['contract_name', 'contract_purpose', 'version'],
    additionalProperties: true
  }));

  fs.writeFileSync(invalidManifestPath, JSON.stringify({
    contract_name: 123,
    contract_purpose: 'broken',
    version: { current: '1.0.0', schema: '1.0.0' }
  }));

  const result = manifestValidator.validateManifestFile(invalidManifestPath, schemaPath);

  assert.equal(result.ok, false, 'malformed manifest should fail validation');
  assert.ok(Array.isArray(result.errors) && result.errors.length > 0, 'error details should be returned');
});
