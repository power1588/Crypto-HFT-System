#!/bin/bash
# Deployment script for Crypto HFT System

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="crypto-hft"
BUILD_TYPE="${1:-release}"
CONFIG_FILE="${2:-config/config.toml}"

echo -e "${GREEN}Starting deployment of ${PROJECT_NAME}...${NC}"

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust.${NC}"
    exit 1
fi

if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${YELLOW}Warning: Config file $CONFIG_FILE not found. Using defaults.${NC}"
fi

# Build
echo -e "${YELLOW}Building project...${NC}"
if [ "$BUILD_TYPE" = "release" ]; then
    cargo build --release
    BINARY_PATH="target/release/crypto_hft"
else
    cargo build
    BINARY_PATH="target/debug/crypto_hft"
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Build failed. Binary not found.${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"

# Run tests
echo -e "${YELLOW}Running tests...${NC}"
if cargo test --release; then
    echo -e "${GREEN}All tests passed!${NC}"
else
    echo -e "${RED}Tests failed. Aborting deployment.${NC}"
    exit 1
fi

# Create necessary directories
echo -e "${YELLOW}Creating directories...${NC}"
mkdir -p logs
mkdir -p config

# Check environment variables
echo -e "${YELLOW}Checking environment variables...${NC}"
if [ -z "$BINANCE_API_KEY" ] && [ -z "$OKX_API_KEY" ]; then
    echo -e "${YELLOW}Warning: No API keys found in environment. Using test mode.${NC}"
fi

# Deploy
echo -e "${GREEN}Deployment ready!${NC}"
echo -e "${YELLOW}To run the application:${NC}"
echo "  $BINARY_PATH --config $CONFIG_FILE"

# Optional: Run immediately
read -p "Do you want to run the application now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${GREEN}Starting application...${NC}"
    exec "$BINARY_PATH" --config "$CONFIG_FILE"
fi

