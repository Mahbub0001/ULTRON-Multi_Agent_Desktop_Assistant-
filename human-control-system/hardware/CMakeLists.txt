cmake_minimum_required(VERSION 3.16)

project(MarkLIV_Firmware
    VERSION 1.0.0
    LANGUAGES C ASM
    DESCRIPTION "Mark-LIV Human Control System Firmware for Teensy 4.0/4.1"
)

set(CMAKE_C_STANDARD 99)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

if(NOT CMAKE_TOOLCHAIN_FILE)
    set(CMAKE_SYSTEM_NAME Generic)
    set(CMAKE_SYSTEM_PROCESSOR arm)
    
    set(TOOLCHAIN_PREFIX "arm-none-eabi" CACHE STRING "Toolchain prefix")
    
    set(CMAKE_C_COMPILER ${TOOLCHAIN_PREFIX}-gcc)
    set(CMAKE_ASM_COMPILER ${TOOLCHAIN_PREFIX}-gcc)
    set(CMAKE_OBJCOPY ${TOOLCHAIN_PREFIX}-objcopy)
    set(CMAKE_OBJDUMP ${TOOLCHAIN_PREFIX}-objdump)
    set(CMAKE_SIZE ${TOOLCHAIN_PREFIX}-size)
endif()

set(BOARD "TEENSY40" CACHE STRING "Target board: TEENSY40 or TEENSY41")
set(TEENSY_MODEL "TEENSY_MODEL_40" CACHE STRING "Teensy model define")

if(BOARD STREQUAL "TEENSY41")
    set(TEENSY_MODEL "TEENSY_MODEL_41")
    set(FLASH_SIZE_KB 7936)
    set(RAM_SIZE_KB 1024)
    set(LINKER_SCRIPT ${CMAKE_SOURCE_DIR}/firmware/teensy41.ld)
else()
    set(FLASH_SIZE_KB 1984)
    set(RAM_SIZE_KB 1024)
    set(LINKER_SCRIPT ${CMAKE_SOURCE_DIR}/firmware/teensy40.ld)
endif()

set(CPU_FREQUENCY_MHZ 600)
set(F_CPU 600000000)

set(COMMON_FLAGS
    "-mcpu=cortex-m7"
    "-mthumb"
    "-mfloat-abi=hard"
    "-mfpu=fpv5-d16"
    "-D__IMXRT1062__"
    "-DTEENSYDUINO=158"
    "-DARDUINO=10816"
    "-DF_CPU=${F_CPU}"
    "-DLAYOUT_US_ENGLISH"
    "-DTEENSY_MODEL=${TEENSY_MODEL}"
    "-DFLASH_SIZE_KB=${FLASH_SIZE_KB}"
    "-DRAM_SIZE_KB=${RAM_SIZE_KB}"
    "-DCPU_FREQUENCY_MHZ=${CPU_FREQUENCY_MHZ}"
    "-ffunction-sections"
    "-fdata-sections"
    "-fno-common"
    "-fno-exceptions"
    "-fno-rtti"
    "-fno-threadsafe-statics"
    "-fno-use-cxa-atexit"
)

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${COMMON_FLAGS} -std=c99 -Wall -Wextra -Werror -Wno-unused-parameter -Wno-unused-function -Wno-missing-field-initializers -Os -g")
set(CMAKE_ASM_FLAGS "${CMAKE_ASM_FLAGS} ${COMMON_FLAGS}")

set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -specs=nano.specs -specs=nosys.specs -Wl,--gc-sections -Wl,--print-memory-usage -T ${LINKER_SCRIPT}")

set(SOURCES
    firmware/main.c
    firmware/usb/descriptors.c
    firmware/usb/callbacks.c
    firmware/usb/core.c
    firmware/protocol/parser.c
    firmware/protocol/handler.c
    firmware/protocol/ack.c
    firmware/input/keyboard.c
    firmware/input/mouse.c
    firmware/input/consumer.c
    firmware/input/system.c
)

add_executable(firmware ${SOURCES})

target_include_directories(firmware PRIVATE
    firmware
    firmware/usb
    firmware/protocol
    firmware/input
)

set_target_properties(firmware PROPERTIES
    OUTPUT_NAME "firmware"
    SUFFIX ".elf"
)

add_custom_command(TARGET firmware POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -O ihex -R .eeprom $<TARGET_FILE:firmware> $<TARGET_FILE_DIR:firmware>/firmware.hex
    COMMAND ${CMAKE_OBJCOPY} -O binary $<TARGET_FILE:firmware> $<TARGET_FILE_DIR:firmware>/firmware.bin
    COMMAND ${CMAKE_OBJDUMP} -h -S $<TARGET_FILE:firmware> > $<TARGET_FILE_DIR:firmware>/firmware.lst
    COMMAND ${CMAKE_SIZE} -A $<TARGET_FILE:firmware>
    COMMAND ${CMAKE_SIZE} -B $<TARGET_FILE:firmware>
    COMMENT "Generating firmware artifacts"
)

add_custom_target(flash
    COMMAND teensy_loader_cli -mmcu=IMXRT1062 -w -v $<TARGET_FILE_DIR:firmware>/firmware.hex
    DEPENDS firmware
    COMMENT "Flashing firmware to Teensy"
)

add_custom_target(size
    COMMAND ${CMAKE_SIZE} -A $<TARGET_FILE:firmware>
    COMMAND ${CMAKE_SIZE} -B $<TARGET_FILE:firmware>
    DEPENDS firmware
    COMMENT "Showing firmware size"
)

add_custom_target(clean_all
    COMMAND ${CMAKE_COMMAND} -E remove_directory ${CMAKE_BINARY_DIR}
    COMMENT "Cleaning build directory"
)

enable_testing()