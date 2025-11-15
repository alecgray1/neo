#!/usr/bin/env bash
# Auto-detect Docker bridge interface and run Neo BACnet application
#
# This script automatically finds the Docker bridge network (10.0.1.0/24)
# and sets the NEO_BACNET_IFACE environment variable before running the app.

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔍 Auto-detecting Docker bridge interface...${NC}"

# Find the bridge interface with 10.0.1.1/24 IP
BRIDGE=$(ip addr show | grep "inet 10.0.1.1/24" | awk '{print $NF}' || true)

if [ -z "$BRIDGE" ]; then
    echo -e "${RED}❌ Error: Could not find Docker bridge interface with 10.0.1.1/24${NC}"
    echo -e "${YELLOW}💡 Make sure Docker containers are running:${NC}"
    echo -e "   cd bacnet-test-devices && docker-compose -f docker-compose-bridge.yml up -d"
    exit 1
fi

echo -e "${GREEN}✓ Found Docker bridge: ${BRIDGE}${NC}"

# Set the environment variable
export NEO_BACNET_IFACE="$BRIDGE"

# Also set the port (though it defaults to 47808 now)
export NEO_BACNET_LOCAL_PORT=47808

echo -e "${GREEN}📡 Starting Neo with:${NC}"
echo -e "   NEO_BACNET_IFACE=${BRIDGE}"
echo -e "   NEO_BACNET_LOCAL_PORT=47808"
echo ""

# Run the application
# Use the first argument to choose what to run, default to main app
case "${1:-main}" in
    main)
        echo -e "${GREEN}🚀 Running main application...${NC}"
        cargo run
        ;;
    bacnet_real)
        echo -e "${GREEN}🚀 Running bacnet_real example...${NC}"
        cargo run --example bacnet_real
        ;;
    release)
        echo -e "${GREEN}🚀 Running release build...${NC}"
        cargo run --release
        ;;
    *)
        echo -e "${GREEN}🚀 Running: $@${NC}"
        "$@"
        ;;
esac
