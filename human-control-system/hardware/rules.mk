# Rules.mk - Common build rules for Mark-LIV firmware
# Include this in sub-makefiles or use directly

# Toolchain configuration
TOOLCHAIN_PREFIX ?= arm-none-eabi
CC := $(TOOLCHAIN_PREFIX)-gcc
CXX := $(TOOLCHAIN_PREFIX)-g++
LD := $(TOOLCHAIN_PREFIX)-gcc
OBJCOPY := $(TOOLCHAIN_PREFIX)-objcopy
OBJDUMP := $(TOOLCHAIN_PREFIX)-objdump
SIZE := $(TOOLCHAIN_PREFIX)-size
GDB := $(TOOLCHAIN_PREFIX)-gdb
NM := $(TOOLCHAIN_PREFIX)-nm
READELF := $(TOOLCHAIN_PREFIX)-readelf

# Target configuration
MCU ?= IMXRT1062
F_CPU ?= 600000000
BOARD ?= TEENSY40

ifeq ($(BOARD),TEENSY41)
    TEENSY_MODEL := TEENSY_MODEL_41
    FLASH_SIZE_KB := 7936
    RAM_SIZE_KB := 1024
    LINKER_SCRIPT := firmware/teensy41.ld
else
    TEENSY_MODEL := TEENSY_MODEL_40
    FLASH_SIZE_KB := 1984
    RAM_SIZE_KB := 1024
    LINKER_SCRIPT := firmware/teensy40.ld
endif

# Compiler flags
ARCH_FLAGS := -mcpu=cortex-m7 -mthumb -mfloat-abi=hard -mfpu=fpv5-d16
DEFINES := -D__IMXRT1062__ -DTEENSYDUINO=158 -DARDUINO=10816 -DF_CPU=$(F_CPU) -DLAYOUT_US_ENGLISH
DEFINES += -DTEENSY_MODEL=$(TEENSY_MODEL) -DFLASH_SIZE_KB=$(FLASH_SIZE_KB) -DRAM_SIZE_KB=$(RAM_SIZE_KB)
DEFINES += -DCPU_FREQUENCY_MHZ=600

OPTIMIZATION_FLAGS := -Os -ffunction-sections -fdata-sections
DEBUG_FLAGS := -g -gdwarf-4
WARNING_FLAGS := -Wall -Wextra -Werror -Wno-unused-parameter -Wno-unused-function -Wno-missing-field-initializers
C_STANDARD := -std=c99

CFLAGS := $(ARCH_FLAGS) $(DEFINES) $(OPTIMIZATION_FLAGS) $(DEBUG_FLAGS) $(WARNING_FLAGS) $(C_STANDARD)
CXXFLAGS := $(ARCH_FLAGS) $(DEFINES) $(OPTIMIZATION_FLAGS) $(DEBUG_FLAGS) $(WARNING_FLAGS) -std=c++17 -fno-rtti -fno-exceptions
ASFLAGS := $(ARCH_FLAGS) $(DEFINES) $(DEBUG_FLAGS) -x assembler-with-cpp

LDFLAGS := $(ARCH_FLAGS) -specs=nano.specs -specs=nosys.specs -Wl,--gc-sections -Wl,--print-memory-usage
LDFLAGS += -T $(LINKER_SCRIPT)
LDFLAGS += -Wl,-Map=$(BUILD_DIR)/firmware.map

LIBS := -lm -lc -lgcc -lnosys

# Include paths
INCLUDE_DIRS := \
    -I. \
    -Ifirmware \
    -Ifirmware/usb \
    -Ifirmware/protocol \
    -Ifirmware/input \
    -Ifirmware/../../../teensy4_core

CFLAGS += $(INCLUDE_DIRS)
CXXFLAGS += $(INCLUDE_DIRS)
ASFLAGS += $(INCLUDE_DIRS)

# Build directory
BUILD_DIR ?= build

# Source files
SRC_C := \
    firmware/main.c \
    firmware/usb/descriptors.c \
    firmware/usb/callbacks.c \
    firmware/usb/core.c \
    firmware/protocol/parser.c \
    firmware/protocol/handler.c \
    firmware/protocol/ack.c \
    firmware/input/keyboard.c \
    firmware/input/mouse.c \
    firmware/input/consumer.c \
    firmware/input/system.c \

OBJ_C := $(SRC_C:firmware/%.c=$(BUILD_DIR)/%.o)
DEP_C := $(OBJ_C:.o=.d)

# Default target
all: $(BUILD_DIR)/firmware.hex $(BUILD_DIR)/firmware.bin

# Pattern rules
$(BUILD_DIR)/%.o: firmware/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -MMD -MP -c $< -o $@

$(BUILD_DIR)/%.o: firmware/%.S
	@mkdir -p $(dir $@)
	$(CC) $(ASFLAGS) -MMD -MP -c $< -o $@

# Link
$(BUILD_DIR)/firmware.elf: $(OBJ_C)
	$(LD) $(LDFLAGS) -o $@ $^ $(LIBS)

# Generate outputs
$(BUILD_DIR)/firmware.hex: $(BUILD_DIR)/firmware.elf
	$(OBJCOPY) -O ihex -R .eeprom $< $@

$(BUILD_DIR)/firmware.bin: $(BUILD_DIR)/firmware.elf
	$(OBJCOPY) -O binary $< $@

$(BUILD_DIR)/firmware.lst: $(BUILD_DIR)/firmware.elf
	$(OBJDUMP) -h -S $< > $@

# Size report
size: $(BUILD_DIR)/firmware.elf
	$(SIZE) -A $<
	$(SIZE) -B $<

# Flash
flash: $(BUILD_DIR)/firmware.hex
	teensy_loader_cli -mmcu=$(MCU) -w -v $<

# Debug
debug: $(BUILD_DIR)/firmware.elf
	$(GDB) $<

# Clean
clean:
	rm -rf $(BUILD_DIR)

# Include dependency files
-include $(DEP_C)

.PHONY: all clean flash size debug