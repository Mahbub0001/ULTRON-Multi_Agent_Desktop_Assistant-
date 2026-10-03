@echo off
title MARK-LIV (JARVIS)
chcp 65001 >nul

echo ========================================================
echo               MARK-LIV (JARVIS) Launcher
echo ========================================================
echo.

:: Always resolve files relative to this launcher.
cd /d "%~dp0"

:: Validate that main.py is found
if not exist "main.py" (
    echo [ERROR] main.py could not be found!
    echo Current folder: %CD%
    echo Expected: main.py next to this launcher
    echo.
    pause
    exit /b 1
)

:: Prefer the project environment so dependencies match the application.
if exist "%~dp0.venv\Scripts\python.exe" (
    set "ULTRON_PYTHON=%~dp0.venv\Scripts\python.exe"
    goto python_ready
)

:: Fall back to Python on PATH when no project environment exists.
where python >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Python is not installed or not in your system PATH!
    echo Please install Python or add it to PATH.
    echo.
    pause
    exit /b 1
)
set "ULTRON_PYTHON=python"

:python_ready

echo Starting JARVIS...
echo Working directory: %CD%
echo.

:: Launch the main application
"%ULTRON_PYTHON%" main.py

:: If the application exits or crashes, keep console open to read errors
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ========================================================
    echo [!] JARVIS closed with exit code %ERRORLEVEL%.
    echo ========================================================
    echo.
    pause
)
