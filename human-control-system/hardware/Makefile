MCU = IMXRT1062
F_CPU = 600000000
ARCH = arm
BOARD = TEENSY40

ifeq ($(BOARD),TEENSY41)
    MCU = IMXRT1062
    F_CPU = 600000000
    FLASH_SIZE = 8192K
    RAM_SIZE = 1024K
    LINKER_SCRIPT = firmware/teensy41.ld
else
    MCU = IMXRT1062
    F_CPU = 600000000
    FLASH_SIZE = 2048K
    RAM_SIZE = 1024K
    LINKER_SCRIPT = firmware/teensy40.ld
endif

TOOLCHAIN = arm-none-eabi
CC = $(TOOLCHAIN)-gcc
CXX = $(TOOLCHAIN)-g++
LD = $(TOOLCHAIN)-gcc
OBJCOPY = $(TOOLCHAIN)-objcopy
OBJDUMP = $(TOOLCHAIN)-objdump
SIZE = $(TOOLCHAIN)-size
GDB = $(TOOLCHAIN)-gdb

CFLAGS = -std=c99
CFLAGS += -Wall -Wextra -Werror
CFLAGS += -Wno-unused-parameter
CFLAGS += -Wno-unused-function
CFLAGS += -Wno-missing-field-initializers
CFLAGS += -Os
CFLAGS += -g
CFLAGS += -ffunction-sections
CFLAGS += -fdata-sections
CFLAGS += -fno-common
CFLAGS += -fno-exceptions
CFLAGS += -fno-rtti
CFLAGS += -fno-threadsafe-statics
CFLAGS += -fno-use-cxa-atexit
CFLAGS += -MMD
CFLAGS += -MP
CFLAGS += -mcpu=cortex-m7
CFLAGS += -mthumb
CFLAGS += -mfloat-abi=hard
CFLAGS += -mfpu=fpv5-d16
CFLAGS += -D__IMXRT1062__
CFLAGS += -DTEENSYDUINO=158
CFLAGS += -DARDUINO=10816
CFLAGS += -DF_CPU=$(F_CPU)
CFLAGS += -DLAYOUT_US_ENGLISH
CFLAGS += -I.
CFLAGS += -Ifirmware
CFLAGS += -Ifirmware/usb
CFLAGS += -Ifirmware/protocol
CFLAGS += -Ifirmware/input
CFLAGS += -Ifirmware/../../../teensy4_core

LDFLAGS = -mcpu=cortex-m7
LDFLAGS += -mthumb
LDFLAGS += -mfloat-abi=hard
LDFLAGS += -mfpu=fpv5-d16
LDFLAGS += -specs=nano.specs
LDFLAGS += -specs=nosys.specs
LDFLAGS += -Wl,--gc-sections
LDFLAGS += -Wl,--print-memory-usage
LDFLAGS += -T $(LINKER_SCRIPT)
LDFLAGS += -Lfirmware/../../../teensy4_core

LIBS = -lm -lc -lgcc -lnosys

SRC_DIR = firmware
BUILD_DIR = build

SOURCES = \
    $(SRC_DIR)/main.c \
    $(SRC_DIR)/usb/descriptors.c \
    $(SRC_DIR)/usb/callbacks.c \
    $(SRC_DIR)/usb/core.c \
    $(SRC_DIR)/protocol/parser.c \
    $(SRC_DIR)/protocol/handler.c \
    $(SRC_DIR)/protocol/ack.c \
    $(SRC_DIR)/input/keyboard.c \
    $(SRC_DIR)/input/mouse.c \
    $(SRC_DIR)/input/consumer.c \
    $(SRC_DIR)/input/system.c \

OBJECTS = $(SOURCES:$(SRC_DIR)/%.c=$(BUILD_DIR)/%.o)
DEPS = $(OBJECTS:.o=.d)

TARGET = $(BUILD_DIR)/firmware
ELF = $(TARGET).elf
HEX = $(TARGET).hex
BIN = $(TARGET).bin
LST = $(TARGET).lst
MAP = $(TARGET).map

.PHONY: all clean flash size debug

all: $(HEX) $(BIN) size

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

$(ELF): $(OBJECTS)
	$(LD) $(LDFLAGS) -o $@ $^ $(LIBS) -Wl,-Map=$(MAP)

$(HEX): $(ELF)
	$(OBJCOPY) -O ihex -R .eeprom $< $@

$(BIN): $(ELF)
	$(OBJCOPY) -O binary $< $@

$(LST): $(ELF)
	$(OBJDUMP) -h -S $< > $@

size: $(ELF)
	$(SIZE) -A $<
	$(SIZE) -B $<

clean:
	rm -rf $(BUILD_DIR)

flash: $(HEX)
	teensy_loader_cli -mmcu=$(MCU) -w -v $<

debug: $(ELF)
	$(GDB) $<

-include $(DEPS)