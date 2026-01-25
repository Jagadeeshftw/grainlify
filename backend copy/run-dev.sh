#!/bin/bash
# Development server with auto-reload using air
# This script will automatically restart the server when files change

# Add ~/go/bin to PATH if not already there
if [[ ":$PATH:" != *":$HOME/go/bin:"* ]]; then
    export PATH="$PATH:$HOME/go/bin"
fi

# Check if air is installed
if ! command -v air &> /dev/null; then
    echo "Air is not installed. Installing..."
    go install github.com/air-verse/air@latest
    
    # Add to PATH again after installation
    if [[ ":$PATH:" != *":$HOME/go/bin:"* ]]; then
        export PATH="$PATH:$HOME/go/bin"
    fi
    
    # Check again
    if ! command -v air &> /dev/null; then
        echo ""
        echo "⚠️  Error: Air installation failed or ~/go/bin is not in PATH"
        echo "Please add this to your ~/.zshrc or ~/.bashrc:"
        echo "  export PATH=\$PATH:\$HOME/go/bin"
        echo ""
        echo "Then run this script again."
        exit 1
    fi
fi

# Run air
echo "🚀 Starting development server with auto-reload..."
echo "📝 Watching for file changes in: $(pwd)"
echo "🛑 Press Ctrl+C to stop"
echo ""

air

