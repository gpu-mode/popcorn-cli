#!/bin/bash

set -e

# Popcorn CLI Hackathon Installer (Unix/Linux/macOS)
# For Windows users: Use install.ps1 instead
echo "🍿 Installing Popcorn CLI for Hackathon (Unix/Linux/macOS)..."

# Check if we're on Windows
if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    echo "⚠️  Detected Windows environment"
    echo "For native Windows, please use install.ps1 instead:"
    echo "   powershell -ExecutionPolicy Bypass -File install.ps1"
    echo ""
    echo "This script will continue assuming you're in a Unix-like environment (WSL/Git Bash/MSYS2)"
    read -p "Continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 0
    fi
fi

# Detect OS
OS=""
ARCH=""
BINARY_NAME=""
EXTENSION=""

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
    EXTENSION=".tar.gz"
    BINARY_NAME="popcorn-cli"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
    EXTENSION=".tar.gz"
    BINARY_NAME="popcorn-cli"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    OS="windows"
    EXTENSION=".zip"
    BINARY_NAME="popcorn-cli.exe"
else
    echo "❌ Unsupported operating system: $OSTYPE"
    exit 1
fi

echo "✅ Detected OS: $OS"

# Download URL
DOWNLOAD_URL="https://github.com/gpu-mode/popcorn-cli/releases/download/v1.1.6/popcorn-cli-${OS}${EXTENSION}"
TEMP_DIR="/tmp/popcorn-cli-install"
INSTALL_DIR="$HOME/.local/bin"

# Create directories
mkdir -p "$TEMP_DIR"
mkdir -p "$INSTALL_DIR"

echo "📥 Downloading from: $DOWNLOAD_URL"

# Download the binary
if command -v curl >/dev/null 2>&1; then
    curl -L -o "$TEMP_DIR/popcorn-cli${EXTENSION}" "$DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
    wget -O "$TEMP_DIR/popcorn-cli${EXTENSION}" "$DOWNLOAD_URL"
else
    echo "❌ Neither curl nor wget found. Please install one of them."
    exit 1
fi

echo "📦 Extracting binary..."

# Extract the binary
cd "$TEMP_DIR"
if [[ "$EXTENSION" == ".tar.gz" ]]; then
    tar -xzf "popcorn-cli${EXTENSION}"
elif [[ "$EXTENSION" == ".zip" ]]; then
    unzip "popcorn-cli${EXTENSION}"
fi

# Find and move the binary
if [[ -f "$BINARY_NAME" ]]; then
    chmod +x "$BINARY_NAME"
    mv "$BINARY_NAME" "$INSTALL_DIR/"
    echo "✅ Binary installed to $INSTALL_DIR/$BINARY_NAME"
else
    echo "❌ Binary not found after extraction"
    exit 1
fi

# Add to PATH
SHELL_RC=""
if [[ -n "$ZSH_VERSION" ]]; then
    SHELL_RC="$HOME/.zshrc"
elif [[ -n "$BASH_VERSION" ]]; then
    SHELL_RC="$HOME/.bashrc"
else
    # Try to detect shell
    case "$SHELL" in
        */zsh)
            SHELL_RC="$HOME/.zshrc"
            ;;
        */bash)
            SHELL_RC="$HOME/.bashrc"
            ;;
        *)
            SHELL_RC="$HOME/.profile"
            ;;
    esac
fi

# Check if PATH already contains the directory
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "🔧 Adding $INSTALL_DIR to PATH in $SHELL_RC"
    echo "" >> "$SHELL_RC"
    echo "# Added by Popcorn CLI installer" >> "$SHELL_RC"
    echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_RC"
    export PATH="$INSTALL_DIR:$PATH"
else
    echo "✅ $INSTALL_DIR already in PATH"
fi

# Cleanup
rm -rf "$TEMP_DIR"

# Auto-register with Discord for hackathon
echo "🔐 Setting up authentication..."
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    # Temporarily add to PATH for immediate use
    export PATH="$INSTALL_DIR:$PATH"
fi

# Check if already registered
if [[ -f "$HOME/.popcorn.yaml" ]]; then
    echo "✅ Already registered! Skipping authentication setup."
else
    echo "🚀 Registering with Discord for hackathon access..."
    if "$INSTALL_DIR/$BINARY_NAME" register discord; then
        echo "✅ Registration successful!"
    else
        echo "⚠️  Registration failed. You can register manually later with:"
        echo "   popcorn-cli register discord"
    fi
fi

echo ""
echo "🎉 Popcorn CLI installed and configured for hackathon!"
echo ""
echo "📋 Quick Start:"
echo "   1. Restart your terminal or run: source $SHELL_RC"
echo "   2. Submit your first solution: popcorn-cli submit <your-file>"
echo ""
echo "🚀 Hackathon mode features:"
echo "   - ✅ API URL pre-configured"
echo "   - ✅ Discord authentication set up"
echo "   - ✅ Only 'test' and 'benchmark' modes available"
echo "   - ✅ Ready to use immediately!"
echo ""
echo "💡 Need help? Run: popcorn-cli --help" 