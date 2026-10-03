@echo off
REM flash.bat - Flash firmware to Teensy 4.0/4.1 (Windows)
REM Usage: scripts\flash.bat [firmware.hex] [board]

setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..
set BUILD_DIR=%PROJECT_ROOT%\build

set FIRMWARE_HEX=%~1
set BOARD=%~2

if "%FIRMWARE_HEX%"=="" set FIRMWARE_HEX=%BUILD_DIR%\firmware.hex
if "%BOARD%"=="" set BOARD=TEENSY40

if "%BOARD%"=="TEENSY41" (
    set MCU=IMXRT1062
) else (
    set MCU=IMXRT1062
)

echo === Mark-LIV Firmware Flasher ===
echo Board: %BOARD%
echo MCU: %MCU%
echo Firmware: %FIRMWARE_HEX%

if not exist "%FIRMWARE_HEX%" (
    echo Error: Firmware file not found: %FIRMWARE_HEX%
    echo Build the firmware first with: make
    exit /b 1
)

where teensy_loader_cli >nul 2>nul
if %errorlevel% neq 0 (
    echo Error: teensy_loader_cli not found in PATH
    echo Download from: https://www.pjrc.com/teensy/loader_cli.html
    exit /b 1
)

echo Flashing firmware...
teensy_loader_cli -mmcu=%MCU% -w -v "%FIRMWARE_HEX%"

echo === Flash complete ===