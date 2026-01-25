#!/bin/bash
# Test script using Docker (works around Rust version issues)

set -e

echo "🐳 Building Docker image..."
docker build -f Dockerfile.test -t program-escrow-test .

echo "🧪 Running tests in Docker..."
docker run --rm program-escrow-test

echo "✅ Tests completed successfully!"
