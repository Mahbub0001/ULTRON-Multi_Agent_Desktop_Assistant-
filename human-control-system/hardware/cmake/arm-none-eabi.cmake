# arm-none-eabi.cmake - CMake toolchain for ARM Cortex-M7 (Teensy 4.x)

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR arm)

set(TOOLCHAIN_PREFIX "arm-none-eabi" CACHE STRING "Toolchain prefix")

set(CMAKE_C_COMPILER ${TOOLCHAIN_PREFIX}-gcc)
set(CMAKE_CXX_COMPILER ${TOOLCHAIN_PREFIX}-g++)
set(CMAKE_ASM_COMPILER ${TOOLCHAIN_PREFIX}-gcc)
set(CMAKE_OBJCOPY ${TOOLCHAIN_PREFIX}-objcopy)
set(CMAKE_OBJDUMP ${TOOLCHAIN_PREFIX}-objdump)
set(CMAKE_SIZE ${TOOLCHAIN_PREFIX}-size)
set(CMAKE_NM ${TOOLCHAIN_PREFIX}-nm)
set(CMAKE_READELF ${TOOLCHAIN_PREFIX}-readelf)
set(CMAKE_STRIP ${TOOLCHAIN_PREFIX}-strip)

set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

set(CMAKE_C_FLAGS_INIT
    "-mcpu=cortex-m7"
    "-mthumb"
    "-mfloat-abi=hard"
    "-mfpu=fpv5-d16"
    "-D__IMXRT1062__"
    "-DTEENSYDUINO=158"
    "-DARDUINO=10816"
    "-DF_CPU=600000000"
    "-DLAYOUT_US_ENGLISH"
    "-ffunction-sections"
    "-fdata-sections"
    "-fno-common"
    "-fno-exceptions"
    "-fno-rtti"
    "-fno-threadsafe-statics"
    "-fno-use-cxa-atexit"
    "-std=c99"
    "-Wall"
    "-Wextra"
    "-Werror"
    "-Wno-unused-parameter"
    "-Wno-unused-function"
    "-Wno-missing-field-initializers"
    "-Os"
    "-g"
    "-gdwarf-4"
)

set(CMAKE_CXX_FLAGS_INIT
    "-mcpu=cortex-m7"
    "-mthumb"
    "-mfloat-abi=hard"
    "-mfpu=fpv5-d16"
    "-D__IMXRT1062__"
    "-DTEENSYDUINO=158"
    "-DARDUINO=10816"
    "-DF_CPU=600000000"
    "-DLAYOUT_US_ENGLISH"
    "-ffunction-sections"
    "-fdata-sections"
    "-fno-common"
    "-fno-exceptions"
    "-fno-rtti"
    "-fno-threadsafe-statics"
    "-fno-use-cxa-atexit"
    "-std=c++17"
    "-Wall"
    "-Wextra"
    "-Werror"
    "-Wno-unused-parameter"
    "-Wno-unused-function"
    "-Wno-missing-field-initializers"
    "-Os"
    "-g"
    "-gdwarf-4"
)

set(CMAKE_ASM_FLAGS_INIT
    "-mcpu=cortex-m7"
    "-mthumb"
    "-mfloat-abi=hard"
    "-mfpu=fpv5-d16"
    "-D__IMXRT1062__"
    "-DTEENSYDUINO=158"
    "-DARDUINO=10816"
    "-DF_CPU=600000000"
    "-DLAYOUT_US_ENGLISH"
    "-x assembler-with-cpp"
    "-g"
)

set(CMAKE_EXE_LINKER_FLAGS_INIT
    "-mcpu=cortex-m7"
    "-mthumb"
    "-mfloat-abi=hard"
    "-mfpu=fpv5-d16"
    "-specs=nano.specs"
    "-specs=nosys.specs"
    "-Wl,--gc-sections"
    "-Wl,--print-memory-usage"
)

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)