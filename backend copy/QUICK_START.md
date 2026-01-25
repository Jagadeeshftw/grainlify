# 🚀 Quick Start Guide

## Auto-Reload Development Server

The backend server automatically restarts when you change any `.go` file.

### Start Development Server

```bash
# Option 1: Use the run script (recommended - handles everything automatically)
./run-dev.sh

# Option 2: Use make
make dev

# Option 3: Use air directly (if installed)
air
```

### First Time Setup

If `air` is not installed:

```bash
# Install air
go install github.com/air-verse/air@latest

# Add to PATH (add to ~/.zshrc or ~/.bashrc)
export PATH=$PATH:$HOME/go/bin

# Then run
./run-dev.sh
```

### What Gets Watched

- ✅ All `.go` files in `cmd/`, `internal/`, and root directory
- ✅ Automatically restarts within 1 second of file changes
- ❌ Excludes: `tmp/`, `vendor/`, `testdata/`, `migrations/`, `.git/`, test files

### Configuration

Air configuration is in `.air.toml`. The server will:
- Build: `go build -o ./tmp/main ./cmd/api`
- Run: `./tmp/main`
- Watch: All `.go` files (except tests and excluded directories)

### Stop the Server

Press `Ctrl+C` to stop the development server.

### Troubleshooting

**Air not found:**
```bash
# Install it
go install github.com/air-verse/air@latest

# Make sure ~/go/bin is in PATH
export PATH=$PATH:$HOME/go/bin

# Verify installation
which air
```

**Server not restarting:**
- Check that you're editing `.go` files (not just config files)
- Check `tmp/build-errors.log` for build errors
- Make sure you're in the `backend/` directory

**Port already in use:**
- Change `PORT` in your `.env` file
- Or kill the existing process: `lsof -ti:8080 | xargs kill`















