#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' 

# Configuration
REPO="aelune/kondo"
BINARY_NAME="kondo"
INSTALL_DIR="$HOME/.local/bin"
BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"

echo "Installing ${BINARY_NAME}..."

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux*)
        OS="linux"
        ;;
    darwin*)
        OS="macos"
        ;;
    *)
        echo -e "${RED}Unsupported OS: $OS${NC}"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo -e "${RED}Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

ASSET_NAME="${BINARY_NAME}-${OS}-${ARCH}"
echo "Detected platform: ${OS}-${ARCH}"

# Get latest release URL
echo "Fetching latest release..."
RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
DOWNLOAD_URL=$(curl -s "$RELEASE_URL" | grep "browser_download_url.*${ASSET_NAME}" | cut -d '"' -f 4)

if [ -z "$DOWNLOAD_URL" ]; then
    echo -e "${RED}Could not find release for ${ASSET_NAME}${NC}"
    echo "Please check if the release exists at: https://github.com/${REPO}/releases"
    exit 1
fi

# Create installation directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Download and install
TMP_FILE=$(mktemp)
echo "Downloading ${BINARY_NAME}..."
curl -sL "$DOWNLOAD_URL" -o "$TMP_FILE"

echo "Installing to ${BINARY_PATH}..."
chmod +x "$TMP_FILE"
mv "$TMP_FILE" "${BINARY_PATH}"

echo -e "${GREEN}✓ Successfully installed ${BINARY_NAME} to ${BINARY_PATH}${NC}"

# Check if install directory is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}Warning: ${INSTALL_DIR} is not in your PATH${NC}"
    echo "Add this line to your ~/.bashrc, ~/.zshrc, or ~/.profile:"
    echo -e "${GREEN}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
    echo "Then run: source ~/.bashrc (or your shell config file)"
else
    echo -e "${GREEN}✓ ${INSTALL_DIR} is already in your PATH${NC}"
fi

echo ""
echo "Run '${BINARY_NAME} --version' to verify installation"
