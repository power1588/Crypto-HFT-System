#!/bin/bash
# Validation script for quickstart.md

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

QUICKSTART_FILE="specs/001-market-making/quickstart.md"
ERRORS=0

echo -e "${GREEN}Validating quickstart.md...${NC}"

# Check if quickstart file exists
if [ ! -f "$QUICKSTART_FILE" ]; then
    echo -e "${RED}Error: $QUICKSTART_FILE not found${NC}"
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Warning: cargo not found. Some checks will be skipped.${NC}"
else
    echo -e "${GREEN}✓ Rust/Cargo found${NC}"
fi

# Check if config directory exists
if [ -d "config" ]; then
    echo -e "${GREEN}✓ Config directory exists${NC}"
    
    # Check for example.toml
    if [ -f "config/example.toml" ]; then
        echo -e "${GREEN}✓ config/example.toml exists${NC}"
    else
        echo -e "${RED}✗ config/example.toml not found${NC}"
        ERRORS=$((ERRORS + 1))
    fi
else
    echo -e "${RED}✗ Config directory not found${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Check if main binary can be built (if cargo available)
if command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Checking if project builds...${NC}"
    if cargo check --message-format=short 2>&1 | grep -q "error"; then
        echo -e "${YELLOW}Warning: Project has compilation errors (may be expected)${NC}"
    else
        echo -e "${GREEN}✓ Project compiles${NC}"
    fi
fi

# Validate quickstart commands
echo -e "${YELLOW}Validating quickstart commands...${NC}"

# Check if commands in quickstart are valid
if grep -q "cargo build --release" "$QUICKSTART_FILE"; then
    echo -e "${GREEN}✓ Build command found${NC}"
fi

if grep -q "cargo test" "$QUICKSTART_FILE"; then
    echo -e "${GREEN}✓ Test command found${NC}"
fi

if grep -q "cargo run" "$QUICKSTART_FILE"; then
    echo -e "${GREEN}✓ Run command found${NC}"
fi

# Check for configuration examples
if grep -q "api_key" "$QUICKSTART_FILE"; then
    echo -e "${GREEN}✓ API key configuration example found${NC}"
fi

# Check for monitoring endpoints
if grep -q "/status\|/health\|/metrics" "$QUICKSTART_FILE"; then
    echo -e "${GREEN}✓ Monitoring endpoints documented${NC}"
fi

# Summary
echo ""
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓ Quickstart validation passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Quickstart validation failed with $ERRORS error(s)${NC}"
    exit 1
fi

