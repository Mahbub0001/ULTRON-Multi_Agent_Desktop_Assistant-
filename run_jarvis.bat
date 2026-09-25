@echo off
title MARK-LIV (JARVIS)
chcp 65001 >nul

echo ========================================================
echo               MARK-LIV (JARVIS) Launcher
echo ========================================================
echo.

:: Check if running from inside the project directory
if exist "%~dp0main.py" (
    cd /d "%~dp0"
) else (
    cd /d "E:\github-projects\Mark-LIV"
)

:: Validate that main.py is found
if not exist "main.py" (
    echo [ERROR] main.py could not be found!
    echo Current folder: %CD%
    echo Expected: E:\github-projects\Mark-LIV
    echo.
    pause
    exit /b 1
)

:: Check if Python is available
where python >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Python is not installed or not in your system PATH!
    echo Please install Python or add it to PATH.
    echo.
    pause
    exit /b 1
)

echo Starting JARVIS...
echo Working directory: %CD%
echo.

:: Launch the main application
python main.py

:: If the application exits or crashes, keep console open to read errors
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ========================================================
    echo [!] JARVIS closed with exit code %ERRORLEVEL%.
    echo ========================================================
    echo.
    pause
)
