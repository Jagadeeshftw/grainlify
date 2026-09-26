#!/usr/bin/env node
/**
 * Error-code stability gate.
 *
 * Soroban contracts encode failures as numeric discriminants taken from
 * `#[contracterror]` enums. Those numbers are part of the wire contract: they are
 * what clients, indexers and the SDK match on, and they are what gets persisted in
 * logs. Nothing in Rust stops an author from inserting a variant in the middle of
 * an enum, which silently renumbers every variant after it and changes the meaning
 * of every stored code.
 *
 * This script pins the published numbers:
 *
 *   1. every variant of every error enum must carry an explicit discriminant, so the number is
 *      written down rather than implied by position;
 *   2. a committed fixture (`contracts/scripts/error-codes.json`) records the code of every
 *      variant as it is published today, and has to match the source exactly;
 *   3. changing or removing an existing code fails the run;
 *   4. adding a variant requires regenerating the fixture in the same change, so the new number
 *      is visible in review instead of appearing silently.
 *
 * Usage:
 *   node contracts/scripts/check-error-codes.mjs                 # verify (CI gate)
 *   node contracts/scripts/check-error-codes.mjs --update        # rewrite the fixture
 *   node contracts/scripts/check-error-codes.mjs --root DIR      # scan DIR/contracts, DIR/soroban
 *   node contracts/scripts/check-error-codes.mjs --fixture FILE  # pin a different fixture file
 */

import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const argv = process.argv.slice(2);
const UPDATE = argv.includes('--update');
const rootFlag = argv.indexOf('--root');
const fixtureFlag = argv.indexOf('--fixture');
const REPO_ROOT = rootFlag >= 0 ? argv[rootFlag + 1] : join(HERE, '..', '..');
const FIXTURE_PATH = fixtureFlag >= 0 ? argv[fixtureFlag + 1] : join(HERE, 'error-codes.json');

const SCAN_DIRS = ['contracts', 'soroban'];
const IGNORED_DIRS = new Set(['node_modules', 'target', 'test_snapshots', '__pycache__', '.git', 'dist']);
const IGNORED_FILE_SUFFIXES = ['.bak'];

const VARIANT_RE = /^([A-Z][A-Za-z0-9_]*)\s*(?:=\s*([0-9][0-9_]*))?\s*,?\s*$/;

/**
 * Test-only sources are skipped: they may declare throwaway `#[contracterror]`
 * enums, and their codes are not part of the published surface.
 */
const TEST_PATH_RE = /(^|\/)(tests?|benches)\/|(^|\/)(test_[^/]+|[^/]+_test|tests?)\.rs$/;

function* rustFiles(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (IGNORED_DIRS.has(entry.name)) continue;
      yield* rustFiles(join(dir, entry.name));
      continue;
    }
    if (!entry.isFile() || !entry.name.endsWith('.rs')) continue;
    if (IGNORED_FILE_SUFFIXES.some((s) => entry.name.endsWith(s))) continue;
    yield join(dir, entry.name);
  }
}

/** Strip a trailing `//` comment, ignoring `//` inside string literals. */
function stripLineComment(line) {
  let inString = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (ch === '\\' && inString) {
      i++;
      continue;
    }
    if (ch === '"') inString = !inString;
    if (!inString && ch === '/' && line[i + 1] === '/') return line.slice(0, i);
  }
  return line;
}

/**
 * Parse every `#[contracterror]` enum in `source`.
 * Returns `{ EnumName: { variantName: code | null } }`.
 */
export function parseErrorEnums(source) {
  const lines = source.split(/\r?\n/);
  const found = {};
  let attrs = [];

  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();

    if (trimmed === '' || trimmed.startsWith('//')) continue;
    if (trimmed.startsWith('#[')) {
      attrs.push(trimmed);
      continue;
    }

    const enumMatch = /^(?:pub(?:\s*\([^)]*\))?\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(trimmed);
    if (!enumMatch) {
      attrs = [];
      continue;
    }

    const hasContractErrorAttr = attrs.some((attr) => attr.includes('contracterror'));
    const isReprU32 = attrs.some((attr) => /repr\s*\(\s*u32\s*\)/.test(attr));
    const enumName = enumMatch[1];
    // `#[contracterror]` enums are errors by definition. Some repos declare the
    // same surface as a plain `#[repr(u32)]` enum named `*Error` and pass it to
    // `panic_with_error!`, so those are error codes too.
    const isContractError = hasContractErrorAttr || (isReprU32 && /Error$/.test(enumName));

    // Collect the enum body up to its matching closing brace.
    const body = [];
    let depth = 0;
    let started = false;
    for (let j = i; j < lines.length; j++) {
      const line = lines[j];
      for (const ch of line) {
        if (ch === '{') {
          depth++;
          started = true;
        } else if (ch === '}') {
          depth--;
        }
      }
      if (j > i) body.push(line);
      if (started && depth <= 0) break;
      if (!started && j > i) break;
    }

    if (isContractError) {
      const variants = {};
      for (const raw of body) {
        const line = stripLineComment(raw).trim();
        if (line === '' || line.startsWith('#[') || line.startsWith('///') || line === '}' || line === '{') continue;
        const match = VARIANT_RE.exec(line);
        if (!match) continue;
        variants[match[1]] = match[2] === undefined ? null : Number(match[2].replace(/_/g, ''));
      }
      if (Object.keys(variants).length > 0) found[enumName] = variants;
    }

    attrs = [];
    // Skip past the enum body we just consumed.
    let depth2 = 0;
    let seen = false;
    let k = i;
    for (; k < lines.length; k++) {
      for (const ch of lines[k]) {
        if (ch === '{') {
          depth2++;
          seen = true;
        } else if (ch === '}') {
          depth2--;
        }
      }
      if (seen && depth2 <= 0) break;
    }
    i = k;
  }

  return found;
}

/** Scan the tree and return `{ "<relative path>::<Enum>": { variant: code } }`. */
export function collectEnums(root) {
  const result = {};
  for (const dir of SCAN_DIRS) {
    for (const file of rustFiles(join(root, dir))) {
      const key0 = relative(root, file).split(sep).join('/');
      if (TEST_PATH_RE.test(key0)) continue;
      const source = readFileSync(file, 'utf8');
      if (!source.includes('contracterror')) continue;
      const enums = parseErrorEnums(source);
      for (const [name, variants] of Object.entries(enums)) {
        const key = `${key0}::${name}`;
        result[key] = variants;
      }
    }
  }
  return Object.fromEntries(Object.entries(result).sort(([a], [b]) => a.localeCompare(b)));
}

function readFixture() {
  try {
    return JSON.parse(readFileSync(FIXTURE_PATH, 'utf8'));
  } catch {
    return { version: 1, enums: {} };
  }
}

function sortVariants(variants) {
  return Object.fromEntries(Object.entries(variants).sort(([, a], [, b]) => a - b));
}

function main() {
  if (!statSync(REPO_ROOT, { throwIfNoEntry: false })) {
    console.error(`error: scan root does not exist: ${REPO_ROOT}`);
    process.exit(2);
  }

  const current = collectEnums(REPO_ROOT);

  if (UPDATE) {
    const payload = {
      version: 1,
      note: 'Error discriminants published by contract error enums. Regenerate with: node contracts/scripts/check-error-codes.mjs --update',
      enums: Object.fromEntries(
        Object.entries(current).map(([key, variants]) => [key, sortVariants(variants)]),
      ),
    };
    mkdirSync(dirname(FIXTURE_PATH), { recursive: true });
    writeFileSync(FIXTURE_PATH, `${JSON.stringify(payload, null, 2)}\n`);
    const enumCount = Object.keys(current).length;
    const variantCount = Object.values(current).reduce((n, v) => n + Object.keys(v).length, 0);
    console.log(`wrote ${relative(REPO_ROOT, FIXTURE_PATH)}: ${enumCount} enums, ${variantCount} variants`);
    return;
  }

  const fixture = readFixture().enums ?? {};
  const errors = [];

  // 1. Every variant must state its discriminant explicitly.
  for (const [key, variants] of Object.entries(current)) {
    const implicit = Object.entries(variants).filter(([, code]) => code === null).map(([name]) => name);
    if (implicit.length > 0) {
      errors.push(
        `${key}: ${implicit.length} variant(s) have no explicit discriminant (${implicit.slice(0, 6).join(', ')}${implicit.length > 6 ? ', ...' : ''}). Add "= <n>" so the published code is written down.`,
      );
    }
    const seen = new Map();
    for (const [name, code] of Object.entries(variants)) {
      if (code === null) continue;
      if (seen.has(code)) {
        errors.push(`${key}: duplicate discriminant ${code} shared by ${seen.get(code)} and ${name}`);
      }
      seen.set(code, name);
    }
  }

  // 2. Published codes may not change or disappear.
  for (const [key, variants] of Object.entries(fixture)) {
    const live = current[key];
    if (!live) {
      errors.push(`${key}: enum was removed but its codes are published in error-codes.json`);
      continue;
    }
    for (const [name, code] of Object.entries(variants)) {
      if (!(name in live)) {
        errors.push(`${key}::${name}: variant was removed or renamed, but code ${code} is published`);
        continue;
      }
      const now = live[name];
      if (now !== code) {
        errors.push(
          `${key}::${name}: code changed from ${code} to ${now === null ? 'an implicit value' : now}. Published error codes are part of the contract; adding a variant must not renumber existing ones.`,
        );
      }
    }
  }

  // 3. The fixture has to keep recording everything that is published, so a new enum or a new
  //    variant has to show up in the diff rather than slip in unnoticed.
  for (const [key, variants] of Object.entries(current)) {
    const known = fixture[key];
    if (!known) {
      errors.push(
        `${key}: enum is not recorded in error-codes.json. Run: node contracts/scripts/check-error-codes.mjs --update`,
      );
      continue;
    }
    const added = Object.keys(variants).filter((name) => !(name in known));
    if (added.length > 0) {
      errors.push(
        `${key}: ${added.length} new variant(s) not recorded in error-codes.json (${added.slice(0, 6).join(', ')}${added.length > 6 ? ', ...' : ''}). Run: node contracts/scripts/check-error-codes.mjs --update`,
      );
    }
  }

  if (errors.length > 0) {
    console.error('\nError-code stability check FAILED:\n');
    for (const error of errors) console.error(`  • ${error}`);
    console.error(`\n${errors.length} problem(s). See contracts/scripts/error-codes.json.`);
    process.exit(1);
  }

  const enumCount = Object.keys(fixture).length;
  const variantCount = Object.values(fixture).reduce((n, v) => n + Object.keys(v).length, 0);
  console.log(`error-code stability check passed (${enumCount} pinned enums, ${variantCount} pinned codes)`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main();
}
