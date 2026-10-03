#!/bin/bash
# build_all.sh - Build firmware for all supported boards
# Usage: ./scripts/build_all.sh [clean]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

BOARDS=("TEENSY40" "TEENSY41")
ACTION="${1:-build}"

echo "=== Mark-LIV Firmware Build Script ==="
echo "Action: $ACTION"
echo "Boards: ${BOARDS[*]}"

for BOARD in "${BOARDS[@]}"; do
    echo ""
    echo "=== Building for $BOARD ==="
    
    BUILD_DIR="${PROJECT_ROOT}/build_${BOARD}"
    
    if [[ "$ACTION" == "clean" ]]; then
        echo "Cleaning $BOARD build..."
        rm -rf "$BUILD_DIR"
        continue
    fi
    
    mkdir -p "$BUILD_DIR"
    
    echo "Configuring..."
    cmake -B "$BUILD_DIR" -S "$PROJECT_ROOT" \
        -DBOARD="$BOARD" \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_TOOLCHAIN_FILE="${PROJECT_ROOT}/cmake/arm-none-eabi.cmake" 2>/dev/null || \
    cmake -B "$BUILD_DIR" -S "$PROJECT_ROOT" -DBOARD="$BOARD"
    
    echo "Building..."
    cmake --build "$BUILD_DIR" --config Release -j$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
    
    echo "Size report for $BOARD:"
    if [[ -f "$BUILD_DIR/firmware.elf" ]]; then
        arm-none-eabi-size -A "$BUILD_DIR/firmware.elf"
        arm-none-eabi-size -B "$BUILD_DIR/firmware.elf"
    fi
    
    echo "Artifacts for $BOARD:"
    ls -lh "$BUILD_DIR"/firmware.* 2>/dev/null || true
done

if [[ "$ACTION" != "clean" ]]; then
    echo ""
    echo "=== Build complete ==="
    echo "Firmware files:"
    find "$PROJECT_ROOT"/build_* -name "firmware.hex" -o -name "firmware.bin" 2>/dev/null | head -20
fi