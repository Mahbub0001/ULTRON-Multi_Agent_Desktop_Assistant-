#!/bin/bash
set -euo pipefail

# HCS Entrypoint Script
# Handles initialization, permission fixes, and service routing

CONFIG_DIR="/etc/hcs"
DATA_DIR="/var/lib/hcs"
LOG_DIR="/var/log/hcs"

# Ensure directories exist with correct permissions
mkdir -p "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"
chmod 755 "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"

# Fix uinput permissions on Linux
if [ -e /dev/uinput ]; then
    chmod 666 /dev/uinput 2>/dev/null || true
fi

# Fix hidraw permissions
for dev in /dev/hidraw*; do
    [ -e "$dev" ] && chmod 666 "$dev" 2>/dev/null || true
done

# Setup X11 authentication for screen capture
if [ -n "${DISPLAY:-}" ] && [ ! -f /root/.Xauthority ]; then
    touch /root/.Xauthority
    xauth generate "$DISPLAY" . trusted 2>/dev/null || true
fi

# Function to wait for dependent services
wait_for_service() {
    local host=$1
    local port=$2
    local name=$3
    local timeout=${4:-30}
    
    echo "Waiting for $name at $host:$port..."
    for i in $(seq 1 $timeout); do
        if nc -z "$host" "$port" 2>/dev/null; then
            echo "$name is ready"
            return 0
        fi
        sleep 1
    done
    echo "ERROR: $name not ready after ${timeout}s"
    return 1
}

# Route to appropriate service
case "${1:-agent}" in
    agent)
        echo "Starting HCS Agent Daemon..."
        exec hcs-agent --config "$CONFIG_DIR/agent.toml"
        ;;
    vision)
        echo "Starting HCS Vision Server..."
        cd /opt/hcs/vision
        exec python3 -m uvicorn server:app --host 0.0.0.0 --port 50051 --workers 4
        ;;
    brain)
        echo "Starting HCS Brain Engine..."
        # Wait for vision service
        wait_for_service localhost 50051 "Vision Server" 60
        exec hcs-brain --config "$CONFIG_DIR/brain.toml"
        ;;
    orchestrator)
        echo "Starting HCS Orchestrator..."
        exec hcs-orchestrator --config "$CONFIG_DIR/orchestrator.toml" "${@:2}"
        ;;
    shell)
        echo "Dropping to shell..."
        exec /bin/bash
        ;;
    *)
        echo "Unknown service: $1"
        echo "Usage: entrypoint.sh [agent|vision|brain|orchestrator|shell]"
        exit 1
        ;;
esac