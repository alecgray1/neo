#!/usr/bin/env bash
# Source this script to set BACnet environment variables
# Usage: source env.sh

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}Auto-detecting Docker bridge interface...${NC}"

# Find the bridge interface with 10.0.1.1/24 IP
BRIDGE=$(ip addr show | grep "inet 10.0.1.1/24" | awk '{print $NF}' || true)

if [ -z "$BRIDGE" ]; then
    echo -e "${RED}Error: Could not find Docker bridge interface with 10.0.1.1/24${NC}"
    echo -e "${YELLOW}Make sure Docker containers are running:${NC}"
    echo -e "   cd /home/alec/Personal/bacnet/docker && docker-compose up -d"
    return 1 2>/dev/null || exit 1
fi

echo -e "${GREEN}Found Docker bridge: ${BRIDGE}${NC}"

# Set environment variables
export NEO_BACNET_IFACE="$BRIDGE"
export BACNET_BROADCAST="10.0.1.255"
export RUST_LOG="${RUST_LOG:-neo=info}"

echo -e "${GREEN}Environment set:${NC}"
echo -e "   NEO_BACNET_IFACE=${BRIDGE}"
echo -e "   BACNET_BROADCAST=10.0.1.255"
echo -e "   RUST_LOG=${RUST_LOG}"
echo ""
echo -e "${GREEN}Now run: cargo run${NC}"
