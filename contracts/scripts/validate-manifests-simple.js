#!/usr/bin/env node

// Backward-compatible entrypoint. Rules live in validate-manifests.js.
const { run } = require('./validate-manifests');

process.exitCode = run();
