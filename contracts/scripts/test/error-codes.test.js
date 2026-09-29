'use strict';

/**
 * Tests for the error-code stability gate (`contracts/scripts/check-error-codes.mjs`).
 *
 * The gate exists because the numeric discriminants of contract error enums are part of the
 * wire contract: clients, indexers and logs match on them. Each case below pins one way that
 * guarantee can be broken — an implicit discriminant, a renumbered code, a removed variant, a
 * variant added without recording it — and asserts the gate fails.
 *
 * The repository-wide case runs the gate against the real tree with the committed fixture, so
 * a stale `contracts/scripts/error-codes.json` fails this suite too.
 */

const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const { pathToFileURL } = require('node:url');

const HERE = __dirname;
const SCRIPT = path.resolve(HERE, '..', 'check-error-codes.mjs');
const REPO_ROOT = process.env.ERROR_CODES_ROOT || path.resolve(HERE, '..', '..');
const FIXTURE = process.env.ERROR_CODES_FIXTURE || path.resolve(HERE, '..', 'error-codes.json');

/** Run the gate. Returns the spawn result so each case can assert on status and output. */
function run(args) {
  return spawnSync(process.execPath, [SCRIPT, ...args], { encoding: 'utf8' });
}

function check(root, fixture) {
  return run(['--root', root, '--fixture', fixture]);
}

function update(root, fixture) {
  return run(['--root', root, '--fixture', fixture, '--update']);
}

/** A throwaway project with `contracts/demo/src/lib.rs` holding `source`. */
function tempProject(source) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'error-codes-'));
  const dir = path.join(root, 'contracts', 'demo', 'src');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'lib.rs'), source);
  return root;
}

function withFixture(root) {
  return path.join(root, 'error-codes.json');
}

function output(result) {
  return `${result.stdout}${result.stderr}`;
}

const SPARSE_ENUM = `#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DemoError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    DeadlineNotPassed = 6,
}
`;

test('the committed fixture matches the error enums in the repository', () => {
  const result = check(REPO_ROOT, FIXTURE);
  assert.equal(result.status, 0, `gate failed:\n${output(result)}`);
  assert.match(result.stdout, /error-code stability check passed/);
});

test('the fixture pins explicit, sparse discriminants rather than positions', () => {
  const fixture = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));

  const programEscrow = fixture.enums['contracts/program-escrow/src/errors.rs::ContractError'];
  assert.ok(programEscrow, 'program-escrow ContractError must be pinned');
  assert.equal(programEscrow.Unauthorized, 1);
  assert.equal(programEscrow.Paused, 3);
  assert.equal(programEscrow.DynamicPricingNotEnabled, 1308);

  // The bounty escrow enum deliberately skips 3-5; a positional parser would report 3 and 4.
  const bountyEscrow = fixture.enums['contracts/bounty_escrow/contracts/escrow/src/lib.rs::Error'];
  assert.ok(bountyEscrow, 'bounty escrow Error must be pinned');
  assert.equal(bountyEscrow.AlreadyInitialized, 1);
  assert.equal(bountyEscrow.DeadlineNotPassed, 6);
  assert.equal(bountyEscrow.EscrowArchived, 61);

  for (const [key, variants] of Object.entries(fixture.enums)) {
    for (const [name, code] of Object.entries(variants)) {
      assert.equal(typeof code, 'number', `${key}::${name} must be pinned to a number`);
    }
  }
});

test('a variant without an explicit discriminant fails the gate', () => {
  const root = tempProject(`#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    Ok = 1,
    Implicit,
}
`);
  const fixture = withFixture(root);
  assert.equal(update(root, fixture).status, 0);

  const result = check(root, fixture);
  assert.equal(result.status, 1);
  assert.match(output(result), /no explicit discriminant/);
  assert.match(output(result), /Implicit/);
});

test('renumbering a published code fails the gate', () => {
  const root = tempProject(`#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    A = 1,
    B = 2,
}
`);
  const fixture = withFixture(root);
  assert.equal(update(root, fixture).status, 0);

  fs.writeFileSync(
    path.join(root, 'contracts', 'demo', 'src', 'lib.rs'),
    `#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    A = 1,
    Inserted = 2,
    B = 3,
}
`,
  );

  const result = check(root, fixture);
  assert.equal(result.status, 1);
  assert.match(output(result), /code changed from 2 to 3/);
});

test('removing or renaming a pinned variant fails the gate', () => {
  const root = tempProject(`#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    A = 1,
    B = 2,
}
`);
  const fixture = withFixture(root);
  assert.equal(update(root, fixture).status, 0);

  fs.writeFileSync(
    path.join(root, 'contracts', 'demo', 'src', 'lib.rs'),
    `#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    A = 1,
}
`,
  );

  const result = check(root, fixture);
  assert.equal(result.status, 1);
  assert.match(output(result), /removed or renamed/);
});

test('a new variant is only accepted once the fixture records it', () => {
  const root = tempProject(`#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    A = 1,
}
`);
  const fixture = withFixture(root);
  assert.equal(update(root, fixture).status, 0);

  fs.writeFileSync(
    path.join(root, 'contracts', 'demo', 'src', 'lib.rs'),
    `#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum DemoError {
    A = 1,
    B = 2,
}
`,
  );

  const stale = check(root, fixture);
  assert.equal(stale.status, 1, 'the fixture has to be regenerated');
  assert.match(output(stale), /new variant\(s\) not recorded/);

  assert.equal(update(root, fixture).status, 0);
  const regenerated = JSON.parse(fs.readFileSync(fixture, 'utf8'));
  assert.equal(regenerated.enums['contracts/demo/src/lib.rs::DemoError'].B, 2);

  const fresh = check(root, fixture);
  assert.equal(fresh.status, 0, output(fresh));
});

test('an unrecorded error enum fails the gate', () => {
  const root = tempProject(SPARSE_ENUM);
  const fixture = withFixture(root);
  assert.equal(update(root, fixture).status, 0);

  const dir = path.join(root, 'soroban', 'contracts', 'demo', 'src');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(
    path.join(dir, 'lib.rs'),
    `#[contracterror]
#[repr(u32)]
pub enum SecondError {
    Only = 1,
}
`,
  );

  const result = check(root, fixture);
  assert.equal(result.status, 1);
  assert.match(output(result), /SecondError: enum is not recorded/);
});

test('test-only sources are not part of the published surface', () => {
  const root = tempProject(SPARSE_ENUM);
  const fixture = withFixture(root);
  assert.equal(update(root, fixture).status, 0);

  const dir = path.join(root, 'contracts', 'demo', 'tests');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(
    path.join(dir, 'demo_test.rs'),
    `#[contracterror]
#[repr(u32)]
pub enum ThrowawayError {
    Nope,
}
`,
  );

  assert.equal(check(root, fixture).status, 0, 'an enum declared under tests/ must be ignored');
});

test('parseErrorEnums recognises contract error enums and ignores unrelated ones', async () => {
  const { parseErrorEnums } = await import(pathToFileURL(SCRIPT).href);

  const parsed = parseErrorEnums(`#[soroban_sdk::contracterror]
#[repr(u32)]
pub enum ContractError {
    A = 1,
    B = 2,
}

#[repr(u32)]
pub enum StatusError {
    Open = 1,
    Closed = 2,
}

#[repr(u32)]
pub enum Status {
    Open = 1,
}

#[derive(Clone)]
pub enum Plain {
    A = 1,
}
`);

  assert.deepEqual(Object.keys(parsed).sort(), ['ContractError', 'StatusError']);
  assert.deepEqual(parsed.ContractError, { A: 1, B: 2 });
  assert.deepEqual(parsed.StatusError, { Open: 1, Closed: 2 });
});
