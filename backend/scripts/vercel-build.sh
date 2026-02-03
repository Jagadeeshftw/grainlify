#!/usr/bin/env bash
set -e
# Install Go on Vercel (not present in default "Other" build image)
GO_VERSION=1.22.4
GO_TAR="go${GO_VERSION}.linux-amd64.tar.gz"
curl -sL "https://go.dev/dl/${GO_TAR}" -o "/tmp/${GO_TAR}"
tar -C /tmp -xzf "/tmp/${GO_TAR}"
export PATH="/tmp/go/bin:${PATH}"
go version
go mod download
go build -o api ./cmd/api
