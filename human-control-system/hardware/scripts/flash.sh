#!/bin/bash
# flash.sh - Flash firmware to Teensy 4.0/4.1
# Usage: ./scripts/flash.sh [firmware.hex] [board]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="${PROJECT_ROOT}/build"

FIRMWARE_HEX="${1:-${BUILD_DIR}/firmware.hex}"
BOARD="${2:-TEENSY40}"

if [[ "$BOARD" == "TEENSY41" ]]; then
    MCU="IMXRT1062"
else
    MCU="IMXRT1062"
fi

echo "=== Mark-LIV Firmware Flasher ==="
echo "Board: $BOARD"
echo "MCU: $MCU"
echo "Firmware: $FIRMWARE_HEX"

if [[ ! -f "$FIRMWARE_HEX" ]]; then
    echo "Error: Firmware file not found: $FIRMWARE_HEX"
    echo "Build the firmware first with: make"
    exit 1
fi

if ! command -v teensy_loader_cli &> /dev/null; then
    echo "Error: teensy_loader_cli not found in PATH"
    echo "Install it with: sudo apt-get install teensy-loader-cli (Linux)"
    echo "Or: brew install teensy_loader_cli (macOS)"
    exit 1
fi

echo "Flashing firmware..."
teensy_loader_cli -mmcu="$MCU" -w -v "$FIRMWARE_HEX"

echo "=== Flash complete ==="