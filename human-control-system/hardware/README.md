# Mark-LIV Hardware Firmware

Firmware for Teensy 4.0/4.1 microcontrollers providing undetectable hardware-level input injection.
Implements standard USB HID (keyboard, mouse, consumer control, system control) plus a custom binary protocol over Raw HID.

## Features

- **USB HID Interfaces**: Keyboard (NKRO), Mouse (relative/absolute), Consumer Control, System Control
- **Raw HID**: Custom binary protocol for programmatic control
- **Custom Protocol**: Binary protocol with acknowledgment, heartbeat, and command/response
- **Bootloader Support**: Enter bootloader mode via command
- **Low Latency**: < 1ms input injection latency

## Hardware Requirements

- Teensy 4.0 (IMXRT1062, 1984KB Flash, 1024KB RAM) or Teensy 4.1 (IMXRT1062, 7936KB Flash, 1024KB RAM)
- ARM Cortex-M7 @ 600 MHz

## Building

### Prerequisites

- ARM GCC toolchain (`arm-none-eabi-gcc` 12+)
- CMake 3.16+
- `teensy_loader_cli` for flashing

### Using Make

```bash
# Build for Teensy 4.0 (default)
make

# Build for Teensy 4.1
make BOARD=TEENSY41

# Clean
make clean

# Flash to device
make flash

# Show size
make size
```

### Using CMake

```bash
# Configure for Teensy 4.0
cmake -B build -S . -DBOARD=TEENSY40 -DCMAKE_TOOLCHAIN_FILE=cmake/arm-none-eabi.cmake

# Configure for Teensy 4.1
cmake -B build -S . -DBOARD=TEENSY41 -DCMAKE_TOOLCHAIN_FILE=cmake/arm-none-eabi.cmake

# Build
cmake --build build --config Release -j$(nproc)

# Flash (requires teensy_loader_cli)
cmake --build build --target flash

# Size report
cmake --build build --target size
```

### Using Build Script

```bash
# Build for all boards
./scripts/build_all.sh

# Clean all builds
./scripts/build_all.sh clean
```

## Protocol

The firmware implements a binary protocol over Raw HID (Report ID 5).

### Packet Format

```
| Magic (4) | Version (1) | Command (1) | Sequence (2) | Length (2) | Payload (N) |
```

### Commands

| Command | Code | Description |
|---------|------|-------------|
| KEY_DOWN | 0x01 | Press a key |
| KEY_UP | 0x02 | Release a key |
| KEY_TAP | 0x03 | Tap a key (press + delay + release) |
| MODIFIER_SET | 0x04 | Set modifier keys |
| MODIFIER_CLEAR | 0x05 | Clear modifier keys |
| MOUSE_MOVE | 0x10 | Relative mouse move |
| MOUSE_MOVE_ABS | 0x11 | Absolute mouse move |
| MOUSE_CLICK | 0x12 | Click mouse button |
| MOUSE_DOWN | 0x13 | Press mouse button |
| MOUSE_UP | 0x14 | Release mouse button |
| MOUSE_WHEEL | 0x15 | Mouse wheel scroll |
| MOUSE_PAN | 0x16 | Mouse horizontal scroll |
| CONSUMER_PRESS | 0x20 | Press consumer key |
| CONSUMER_RELEASE | 0x21 | Release consumer key |
| CONSUMER_TAP | 0x22 | Tap consumer key |
| SYSTEM_PRESS | 0x30 | Press system key |
| SYSTEM_RELEASE | 0x31 | Release system key |
| SYSTEM_TAP | 0x32 | Tap system key |
| RAW_HID_SEND | 0x40 | Send raw HID data |
| HEARTBEAT | 0xF0 | Keepalive |
| RESET | 0xF1 | Reset device |
| GET_VERSION | 0xF2 | Get firmware version |
| GET_STATUS | 0xF3 | Get device status |
| SET_LED | 0xF4 | Set LED state |
| BOOTLOADER | 0xFF | Enter bootloader mode |

### Response Codes

| Code | Name | Description |
|------|------|-------------|
| 0x00 | OK | Success |
| 0x01 | ERROR | Generic error |
| 0x02 | INVALID_CMD | Unknown command |
| 0x03 | INVALID_PARAM | Invalid parameters |
| 0x04 | TIMEOUT | Operation timed out |
| 0x05 | BUSY | Device busy |
| 0x06 | NOT_SUPPORTED | Not supported |
| 0x10 | ACK | Acknowledgment |
| 0x11 | NACK | Negative acknowledgment |
| 0x20 | VERSION | Version response |
| 0x21 | STATUS | Status response |

### Acknowledgment Protocol

Commands that require acknowledgment will receive an ACK packet (RESP_ACK) with the original sequence number.
If no ACK is received within 100ms, the command is retried up to 3 times.

### Heartbeat

The firmware sends a HEARTBEAT command (0xF0) every 1000ms to maintain connection health.

## USB HID Reports

### Keyboard (Report ID 1)
- 8 modifier bits (LCtrl, LShift, LAlt, LGui, RCtrl, RShift, RAlt, RGui)
- 1 reserved byte
- 6 key codes (NKRO supported via bitmap)

### Mouse (Report ID 2)
- 5 buttons (Left, Right, Middle, Back, Forward)
- X, Y, Wheel, Pan (signed 8-bit relative)

### Consumer Control (Report ID 3)
- 16-bit usage code (Volume Up/Down, Mute, Play/Pause, Next/Prev Track, Stop)

### System Control (Report ID 4)
- Power Down, Sleep, Wake Up

### Raw HID (Report ID 5)
- 63 bytes IN/OUT for custom protocol

## Project Structure

```
hardware/
├── Makefile              # Make-based build system
├── CMakeLists.txt        # CMake-based build system
├── rules.mk              # Common build rules
├── cmake/
│   └── arm-none-eabi.cmake  # ARM toolchain file
├── firmware/
│   ├── config.h          # Configuration constants
│   ├── hid_reports.h     # HID report structures
│   ├── protocol.h        # Protocol definitions
│   ├── main.c            # Main entry point
│   ├── teensy40.ld       # Linker script (Teensy 4.0)
│   ├── teensy41.ld       # Linker script (Teensy 4.1)
│   ├── usb/
│   │   ├── descriptors.c # USB descriptors
│   │   └── callbacks.c   # USB callbacks
│   ├── protocol/
│   │   ├── parser.c      # Protocol parser
│   │   ├── handler.c     # Command handlers
│   │   └── ack.c         # Acknowledgment handling
│   └── input/
│       ├── keyboard.c    # Keyboard input handling
│       ├── keyboard.h
│       ├── mouse.c       # Mouse input handling
│       ├── mouse.h
│       ├── consumer.c    # Consumer control handling
│       ├── consumer.h
│       ├── system.c      # System control handling
│       └── system.h
└── scripts/
    ├── flash.sh          # Flash script (Linux/macOS)
    ├── flash.bat         # Flash script (Windows)
    └── build_all.sh      # Build all boards script
```

## Development

### Code Style

- C99 standard
- clang-format for formatting
- Static analysis with cppcheck

### Testing

```bash
# Check formatting
clang-format --dry-run --Werror firmware/**/*.c firmware/**/*.h

# Static analysis
cppcheck --enable=all --std=c99 -I firmware/firmware -I firmware/usb -I firmware/protocol -I firmware/input firmware/
```

## Flashing

### Linux/macOS
```bash
./scripts/flash.sh build/firmware.hex TEENSY40
```

### Windows
```cmd
scripts\flash.bat build\firmware.hex TEENSY40
```

### Manual
```bash
teensy_loader_cli -mmcu=IMXRT1062 -w -v build/firmware.hex
```

## License

Part of the Mark-LIV Human Control System.