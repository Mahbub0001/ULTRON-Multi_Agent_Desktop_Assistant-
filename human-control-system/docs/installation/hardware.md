# Hardware Installation Guide - Teensy Firmware

## Overview

This guide covers setting up the Teensy 4.0/4.1 hardware for undetectable, low-latency input injection. The firmware implements standard USB HID interfaces plus a custom binary protocol over Raw HID.

## Hardware Requirements

### Supported Boards
| Board | MCU | Flash | RAM | Max Speed |
|-------|-----|-------|-----|-----------|
| Teensy 4.0 | IMXRT1062 | 1984 KB | 1024 KB | 600 MHz |
| Teensy 4.1 | IMXRT1062 | 7936 KB | 1024 KB | 600 MHz |

### Required Accessories
- **USB Cable:** Data-capable USB Micro-B (Teensy 4.0) or USB-C (Teensy 4.1)
- **Computer:** Windows, Linux, or macOS for flashing

### Optional Accessories
- **Header Pins:** For breadboard/prototyping
- **MicroSD Card:** Teensy 4.1 only (for logging/storage)
- **External Power:** 5V supply for standalone operation

## 1. Install Build Tools

### Windows
```powershell
# Install ARM GCC toolchain
winget install ARM.ArmGnuToolchain

# Or use Chocolatey
choco install arm-none-eabi-gcc

# Install CMake
winget install Kitware.CMake

# Install teensy_loader_cli
# Download from: https://github.com/PaulStoffregen/teensy_loader_cli/releases
# Or build from source (requires libusb)
```

### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install -y \
    gcc-arm-none-eabi \
    cmake \
    libusb-1.0-0-dev \
    build-essential \
    git

# Install teensy_loader_cli
git clone https://github.com/PaulStoffregen/teensy_loader_cli.git
cd teensy_loader_cli
make
sudo cp teensy_loader_cli /usr/local/bin/
```

### Linux (Fedora)
```bash
sudo dnf install -y \
    arm-none-eabi-gcc-cs \
    arm-none-eabi-newlib \
    cmake \
    libusb-devel \
    gcc-c++ \
    make

# teensy_loader_cli
git clone https://github.com/PaulStoffregen/teensy_loader_cli.git
cd teensy_loader_cli
make
sudo cp teensy_loader_cli /usr/local/bin/
```

### macOS
```bash
# ARM toolchain
brew install arm-none-eabi-gcc

# CMake
brew install cmake

# teensy_loader_cli
brew install teensy_loader_cli
```

## 2. Clone Firmware Repository

```bash
# If part of main repo
cd human-control-system/hardware

# Or standalone
git clone https://github.com/your-org/human-control-system.git
cd human-control-system/hardware
```

## 3. Configure Build

### Using Make (Simplest)

```bash
# Teensy 4.0 (default)
make

# Teensy 4.1
make BOARD=TEENSY41

# Clean build
make clean

# Show memory usage
make size
```

### Using CMake (Advanced)

```bash
# Teensy 4.0
cmake -B build -S . \
    -DBOARD=TEENSY40 \
    -DCMAKE_TOOLCHAIN_FILE=cmake/arm-none-eabi.cmake \
    -DCMAKE_BUILD_TYPE=Release

# Teensy 4.1
cmake -B build -S . \
    -DBOARD=TEENSY41 \
    -DCMAKE_TOOLCHAIN_FILE=cmake/arm-none-eabi.cmake \
    -DCMAKE_BUILD_TYPE=Release

# Build
cmake --build build --config Release -j$(nproc)

# Size report
cmake --build build --target size
```

### Using Build Script (All Boards)

```bash
# Build for both boards
./scripts/build_all.sh

# Clean all
./scripts/build_all.sh clean
```

## 4. Build Configuration Options

### config.h - Key Settings

```c
// hardware/firmware/config.h

// Firmware version
#define FIRMWARE_VERSION_MAJOR 1
#define FIRMWARE_VERSION_MINOR 0
#define FIRMWARE_VERSION_PATCH 0

// USB Identification
#define USB_VID 0x16C0        // Vendor ID (Van Ooijen Technische Informatica)
#define USB_PID 0x0486        // Product ID (Teensy 4.0)
#define USB_MANUFACTURER "Mark-LIV"
#define USB_PRODUCT "HCS Input Device"

// HID Report Configuration
#define KEYBOARD_REPORT_ID 1
#define MOUSE_REPORT_ID 2
#define CONSUMER_REPORT_ID 3
#define SYSTEM_REPORT_ID 4
#define RAW_HID_REPORT_ID 5

// NKRO Configuration
#define NKRO_ENABLED 1
#define NKRO_BITMAP_SIZE 32  // 256 keys

// Protocol Settings
#define PROTOCOL_MAGIC 0x48435301  // "HCS\x01"
#define PROTOCOL_VERSION 1
#define ACK_TIMEOUT_MS 100
#define MAX_RETRIES 3
#define HEARTBEAT_INTERVAL_MS 1000

// LED Configuration
#define LED_PIN 13
#define LED_ACTIVE_LOW 0
```

### Customize VID/PID (Optional)

For production devices, obtain your own VID from usb.org or use pid.codes:

```c
// Use pid.codes (free)
#define USB_VID 0x1209  // pid.codes
#define USB_PID 0xXXXX  // Your assigned PID

// Or use Arduino VID (for development only)
#define USB_VID 0x2341
#define USB_PID 0x0044  // Arduino Pro Micro
```

## 5. Flash Firmware

### Enter Bootloader Mode

1. Connect Teensy via USB
2. Press the **Program button** on the Teensy board
3. LED will blink slowly (bootloader active)

### Flash Using Make

```bash
# Teensy 4.0
make flash

# Teensy 4.1
make BOARD=TEENSY41 flash

# Specify custom hex file
make flash HEX=build/custom_firmware.hex
```

### Flash Using teensy_loader_cli Directly

```bash
# Teensy 4.0
teensy_loader_cli -mmcu=IMXRT1062 -w -v build/firmware.hex

# Teensy 4.1
teensy_loader_cli -mmcu=IMXRT1062 -w -v build/firmware.hex

# Verbose output
teensy_loader_cli -mmcu=IMXRT1062 -w -v -s build/firmware.hex
```

### Flash Using Script

```bash
# Linux/macOS
./scripts/flash.sh build/firmware.hex TEENSY40

# Windows
scripts\flash.bat build\firmware.hex TEENSY40
```

### Verify Flash Success

```
Teensy Loader, Command Line, Version 2.1
Read "build/firmware.hex": 123456 bytes, 6.2% usage
Found HalfKay Bootloader
Programming....................................................
Booting
```

The Teensy will reboot and enumerate as HCS Input Device.

## 6. Verify Device Enumeration

### Windows
```powershell
# Device Manager
# Should show:
# - HCS Input Device (Keyboard)
# - HCS Input Device (Mouse)
# - HCS Input Device (Consumer Control)
# - HCS Input Device (System Control)
# - HCS Input Device (Raw HID)

# PowerShell
Get-PnpDevice -Class HIDClass | Where-Object { $_.FriendlyName -like "*HCS*" }
```

### Linux
```bash
# List HID devices
ls -la /dev/hidraw*

# Check udev
udevadm info -a -n /dev/hidraw0 | grep -E "(VENDOR|PRODUCT|HCS)"

# dmesg
dmesg -T | grep -i teensy
```

### macOS
```bash
# System Profiler
system_profiler SPUSBDataType | grep -A10 "HCS"

# Or IORegistry
ioreg -c IOHIDDevice | grep -i hcs
```

## 7. Test Firmware Communication

### Using Python Test Script

```python
# test_hid.py
import hid
import time

# Find device
device = None
for d in hid.enumerate():
    if d['vendor_id'] == 0x16C0 and d['product_id'] == 0x0486:
        if d['interface_number'] == 4:  # Raw HID interface
            device = hid.Device(path=d['path'])
            break

if not device:
    print("Device not found!")
    exit(1)

print(f"Connected: {device.manufacturer} {device.product}")

# Test handshake
device.write(b"HCS_HID_v1\n")
time.sleep(0.1)
response = device.read(64, timeout_ms=1000)
print(f"Handshake: {response}")

# Test GET_VERSION (0xF2)
device.write(bytes([0x48, 0x43, 0x53, 0x01, 0x01, 0xF2, 0x00, 0x01, 0x00, 0x00]))
time.sleep(0.1)
response = device.read(64, timeout_ms=1000)
print(f"Version: {response.hex()}")

# Test KEY_TAP 'A' (0x04)
device.write(bytes([0x48, 0x43, 0x53, 0x01, 0x01, 0x03, 0x00, 0x02, 0x00, 0x04, 0x00]))
time.sleep(0.1)
response = device.read(64, timeout_ms=1000)
print(f"Key tap: {response.hex()}")

device.close()
```

```bash
pip install hidapi
python test_hid.py
```

### Using Rust Test (from driver crate)

```bash
cd human-control-system/driver
cargo run --example test_hid_device
```

## 8. Protocol Reference

### Packet Format

```
| Magic (4) | Version (1) | Command (1) | Sequence (2) | Length (2) | Payload (N) |
| 0x48435301|     0x01    |    0x01     |   0x0001     |   0x0002   |   0x04 0x00|
```

### Commands

| Command | Code | Payload | Description |
|---------|------|---------|-------------|
| KEY_DOWN | 0x01 | u16 keycode | Press key |
| KEY_UP | 0x02 | u16 keycode | Release key |
| KEY_TAP | 0x03 | u16 keycode, u16 delay_ms | Tap key |
| MODIFIER_SET | 0x04 | u8 modifiers | Set modifiers |
| MODIFIER_CLEAR | 0x05 | u8 modifiers | Clear modifiers |
| MOUSE_MOVE | 0x10 | i16 x, i16 y | Relative move |
| MOUSE_MOVE_ABS | 0x11 | u16 x, u16 y | Absolute move |
| MOUSE_CLICK | 0x12 | u8 button, u16 delay_ms | Click |
| MOUSE_DOWN | 0x13 | u8 button | Press button |
| MOUSE_UP | 0x14 | u8 button | Release button |
| MOUSE_WHEEL | 0x15 | i8 delta | Vertical scroll |
| MOUSE_PAN | 0x16 | i8 delta | Horizontal scroll |
| CONSUMER_PRESS | 0x20 | u16 usage | Media key press |
| CONSUMER_RELEASE | 0x21 | u16 usage | Media key release |
| CONSUMER_TAP | 0x22 | u16 usage, u16 delay | Media key tap |
| SYSTEM_PRESS | 0x30 | u8 usage | System key press |
| SYSTEM_RELEASE | 0x31 | u8 usage | System key release |
| SYSTEM_TAP | 0x32 | u8 usage, u16 delay | System key tap |
| HEARTBEAT | 0xF0 | - | Keepalive |
| RESET | 0xF1 | - | Reset device |
| GET_VERSION | 0xF2 | - | Get firmware version |
| GET_STATUS | 0xF3 | - | Get device status |
| SET_LED | 0xF4 | u8 state | Set LED |
| BOOTLOADER | 0xFF | - | Enter bootloader |

### Key Codes (HID Usage IDs)

Common keys (see `hid_reports.h` for full list):
```
0x04: A        0x1A: W        0x2F: [
0x05: B        0x1B: X        0x30: ]
0x06: C        0x1C: Y        0x31: \
0x07: D        0x1D: Z        0x32: #
0x08: E        0x2C: Space    0x33: ;
0x09: F        0x28: Enter    0x34: '
0x0A: G        0x2A: Backspace 0x35: `
0x0B: H        0x2B: Tab      0x36: ,
0x0C: I        0xE0: L-Ctrl   0x37: .
0x0D: J        0xE1: L-Shift  0x38: /
0x0E: K        0xE2: L-Alt    0x39: CapsLock
0x0F: L        0xE3: L-GUI    0x3A: F1
0x10: M        0xE4: R-Ctrl   ...
0x11: N        0xE5: R-Shift
0x12: O        0xE6: R-Alt
0x13: P        0xE7: R-GUI
0x14: Q        0x2D: Esc
0x15: R
0x16: S
0x17: T
0x18: U
0x19: V
```

### Mouse Buttons
```
0x01: Left
0x02: Right
0x04: Middle
0x08: Back (X1)
0x10: Forward (X2)
```

### Consumer Usages (Media Keys)
```
0xE9: Volume Up
0xEA: Volume Down
0xE2: Mute
0xCD: Play/Pause
0xB5: Next Track
0xB6: Previous Track
0xB7: Stop
```

### System Usages
```
0x81: Power Down
0x82: Sleep
0x83: Wake Up
```

### Response Codes
```
0x00: OK
0x01: ERROR
0x02: INVALID_CMD
0x03: INVALID_PARAM
0x04: TIMEOUT
0x05: BUSY
0x06: NOT_SUPPORTED
0x10: ACK
0x11: NACK
0x20: VERSION response
0x21: STATUS response
```

### Acknowledgment Protocol

Commands requiring ACK:
1. Host sends command with sequence number
2. Device responds with ACK (0x10) + same sequence
3. If no ACK within 100ms → retry (max 3x)
4. After 3 failures → NACK (0x11)

### Heartbeat

- Device sends HEARTBEAT (0xF0) every 1000ms
- Host should respond with ACK
- If 3 heartbeats missed → connection considered lost

## 9. Integration with HCS Agent

### Configure Agent for HID

```toml
# config/agent.toml
[driver]
backend = "hid"
# Auto-detects by VID/PID, or specify:
# port_path = "/dev/ttyACM0"  # Linux
# port_path = "COM3"          # Windows
```

### Test Integration

```bash
# Start agent
cd human-control-system/agent
cargo run --release

# In another terminal, test injection
TOKEN=$(grpcurl -plaintext -d '{"client_id": "test", "capabilities": [{"resource": "input", "actions": ["WRITE"]}]}' localhost:50051 hcs.agent.v1.AuthService/IssueToken | jq -r .token)

# Type "hello"
for key in 0x0B 0x08 0x0F 0x0F 0x12; do  # h e l l o
    grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
        -d "{\"code\": $key, \"state\": \"KEY_STATE_DOWN\"}" \
        localhost:50051 hcs.agent.v1.InputService/InjectKey
    grpcurl -plaintext -H "Authorization: Bearer $TOKEN" \
        -d "{\"code\": $key, \"state\": \"KEY_STATE_UP\"}" \
        localhost:50051 hcs.agent.v1.InputService/InjectKey
done
```

## 10. Troubleshooting

### Device Not Enumerating
- Check USB cable (must be data cable, not charge-only)
- Try different USB port (USB 2.0 vs 3.0)
- Press Program button to enter bootloader, then flash
- Check `dmesg` (Linux) or Device Manager (Windows) for errors

### Build Errors: "arm-none-eabi-gcc not found"
```bash
# Verify installation
arm-none-eabi-gcc --version

# Add to PATH if needed
export PATH="/opt/arm-gnu-toolchain/bin:$PATH"  # Adjust path
```

### Build Errors: "teensy_loader_cli not found"
```bash
# Install or add to PATH
which teensy_loader_cli
# If missing, install per section 1
```

### Flash Fails: "No device found"
- Ensure Teensy is in bootloader mode (press button, LED blinks)
- Run as root/sudo on Linux: `sudo teensy_loader_cli ...`
- Check udev rules for USB access

### Input Not Working After Flash
- Verify device appears as HID keyboard/mouse
- Check agent config uses `backend = "hid"`
- Test with `hidapi` directly (section 7)
- Check agent logs for connection errors

### Latency Issues
- Reduce `ACK_TIMEOUT_MS` in config.h (default 100ms)
- Ensure USB port is not shared with high-bandwidth devices
- Use direct USB connection (no hubs)

### Firmware Crashes / Resets
- Check serial output for panic messages
- Enable debug logging in `config.h`:
  ```c
  #define DEBUG_LOGGING 1
  ```
- Monitor with `teensy_loader_cli -v` or serial terminal

## 11. Development Workflow

### Modify Firmware

```bash
# Edit source files
vim firmware/input/keyboard.c
vim firmware/protocol/handler.c

# Rebuild
make clean
make BOARD=TEENSY40

# Flash and test
make flash
```

### Code Style
```bash
# Format
clang-format -i firmware/**/*.c firmware/**/*.h

# Static analysis
cppcheck --enable=all --std=c99 \
    -I firmware -I firmware/usb -I firmware/protocol -I firmware/input \
    firmware/
```

### Debugging

```bash
# Use J-Link or CMSIS-DAP debugger
# Or add debug prints to firmware

# For serial debug (if using USB Serial):
screen /dev/ttyACM0 115200  # Linux
# Or PuTTY / Terminal on Windows/macOS
```

## 12. Production Considerations

### Security
- Use custom VID/PID (not default Teensy)
- Enable USB device authentication if needed
- Consider firmware signing

### Reliability
- Add watchdog timer
- Implement brownout detection
- Test thermal performance

### Compliance
- USB-IF certification if distributing commercially
- FCC/CE testing for emissions
- RoHS compliance

## Next Steps

- [Windows Installation](../installation/windows.md#8-hardware-teensy-setup)
- [Linux Installation](../installation/linux.md#8-hardware-teensy-setup)
- [macOS Installation](../installation/macos.md#13-hardware-setup-teensy-on-macos)
- [API Reference: HID Driver](../api/agent.md#hid-driver)
- [Architecture: Hardware Integration](../architecture.md#7-hardware-firmware)