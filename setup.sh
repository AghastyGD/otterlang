#!/bin/bash
# OtterLang Setup Script
# Installs the otter CLI globally for easy access

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}🦦 OtterLang Setup${NC}"
echo ""

# Check if Rust/Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo is not installed!${NC}"
    echo -e "${YELLOW}Please install Rust first: https://rustup.rs/${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Cargo found${NC}"

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Check if we're in the otterlang directory
if [ ! -f "$SCRIPT_DIR/Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Cargo.toml not found!${NC}"
    echo -e "${YELLOW}Please run this script from the otterlang project directory${NC}"
    exit 1
fi

echo -e "${BLUE}📦 Building OtterLang...${NC}"
cd "$SCRIPT_DIR"

# Build in release mode
cargo build --release --quiet

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed!${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Build successful${NC}"

# Install the binary
echo -e "${BLUE}🚀 Installing otter command...${NC}"
cargo install --path . --bin otter --force --quiet

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Installation failed!${NC}"
    exit 1
fi

# Check if cargo bin is in PATH
CARGO_BIN="$HOME/.cargo/bin"
if [[ ":$PATH:" != *":$CARGO_BIN:"* ]]; then
    echo ""
    echo -e "${YELLOW}⚠️  Warning: $CARGO_BIN is not in your PATH${NC}"
    echo ""
    echo -e "${CYAN}Add this to your shell config (~/.zshrc or ~/.bashrc):${NC}"
    echo -e "${GREEN}export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
    echo ""
    echo -e "${CYAN}Then run:${NC}"
    echo -e "${GREEN}source ~/.zshrc  # or source ~/.bashrc${NC}"
    echo ""
fi

# Verify installation
if command -v otter &> /dev/null; then
    echo -e "${GREEN}✅ OtterLang installed successfully!${NC}"
    echo ""
    echo -e "${CYAN}🎉 You can now use the 'otter' command anywhere!${NC}"
    echo ""
    echo -e "${BLUE}Try it out:${NC}"
    echo -e "  ${GREEN}otter --help${NC}"
    echo -e "  ${GREEN}otter run examples/hello.otter${NC}"
    echo ""
else
    echo -e "${YELLOW}⚠️  Installation complete, but 'otter' command not found in PATH${NC}"
    echo -e "${CYAN}Make sure $CARGO_BIN is in your PATH${NC}"
    echo ""
    echo -e "${GREEN}You can also run directly:${NC}"
    echo -e "  ${CYAN}$CARGO_BIN/otter --help${NC}"
fi

