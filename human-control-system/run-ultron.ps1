#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Ultron Human Control System - Free Setup & Launch Script (Windows)
.DESCRIPTION
    Sets up and runs the complete Human Control System with free components only.
    Uses synthetic driver (no kernel driver purchase needed), free YOLOv8n model, Tesseract OCR.
.REQUIREMENTS
    - Windows 10/11
    - Docker Desktop installed & running
    - Run as Administrator
#>

param(
    [switch]$SkipBuild,
    [switch]$SkipModels,
    [switch]$NoTUI
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
Set-Location $scriptDir

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "   ULTRON Human Control System - Free Setup  " -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check Admin (warn only, don't fail)
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warning "Not running as Administrator. Some Docker operations may fail. Recommend: Run as Administrator."
    Write-Warning "Continuing anyway in 3 seconds..."
    Start-Sleep 3
}

# Check Docker
Write-Host "[1/6] Checking Docker Desktop..." -ForegroundColor Yellow
try {
    $dockerVersion = docker version --format '{{.Server.Version}}' 2>&1
    if ($LASTEXITCODE -ne 0 -or $dockerVersion -match "cannot") {
        throw "Docker not responding"
    }
    Write-Host "    Docker OK: $dockerVersion" -ForegroundColor Green
} catch {
    Write-Error "ERROR: Docker Desktop not running. Start Docker Desktop first, wait for 'Docker Desktop is running' message."
    exit 1
}

# Create config
Write-Host "[2/6] Creating free config (synthetic driver)..." -ForegroundColor Yellow
$config = @"
[server]
grpc_addr = "0.0.0.0:50051"
http_addr = "0.0.0.0:8080"

[driver]
backend = "Synthetic"
exclusive_mode = false
injection_delay_us = 1000
max_batch_size = 64

[auth]
enabled = false

[logging]
level = "debug"
format = "Text"

[metrics]
enabled = true
addr = "0.0.0.0:9090"
path = "/metrics"
"@
$configDir = Join-Path $scriptDir "config"
if (-not (Test-Path $configDir)) { New-Item -ItemType Directory -Path $configDir | Out-Null }
$config | Out-File (Join-Path $configDir "agent.toml") -Encoding utf8
Write-Host "    Config created: config/agent.toml" -ForegroundColor Green

# Download free models
if (-not $SkipModels) {
    Write-Host "[3/6] Downloading free YOLOv8n model..." -ForegroundColor Yellow
    $modelsDir = Join-Path $scriptDir "vision\models"
    if (-not (Test-Path $modelsDir)) { New-Item -ItemType Directory -Path $modelsDir | Out-Null }
    
    $modelPath = Join-Path $modelsDir "yolov8n.onnx"
    if (-not (Test-Path $modelPath)) {
        try {
            $url = "https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov8n.onnx"
            Write-Host "    Downloading from $url..."
            $progress = $ProgressPreference
            $ProgressPreference = 'SilentlyContinue'
            Invoke-WebRequest -Uri $url -OutFile $modelPath -UseBasicParsing -TimeoutSec 60
            $ProgressPreference = $progress
            Write-Host "    Model saved: $modelPath" -ForegroundColor Green
        } catch {
            Write-Warning "    Model download failed (will use dummy): $_"
        }
    } else {
        Write-Host "    Model already exists" -ForegroundColor Green
    }
}

# Build Docker images
if (-not $SkipBuild) {
    Write-Host "[4/6] Building Docker images (10-15 min first time)..." -ForegroundColor Yellow
    Write-Host "    This compiles Rust, Python, and sets up all services..." -ForegroundColor Gray
    try {
        docker compose -f docker/docker-compose.windows.yml build --parallel
        Write-Host "    Build complete!" -ForegroundColor Green
    } catch {
        Write-Error "Build failed: $_"
        exit 1
    }
}

# Start services
Write-Host "[5/6] Starting services..." -ForegroundColor Yellow
docker compose -f docker/docker-compose.windows.yml up -d
Write-Host "    Services started" -ForegroundColor Green

# Wait for health
Write-Host "    Waiting for services to be healthy..."
$maxWait = 180
$elapsed = 0
while ($elapsed -lt $maxWait) {
    $ps = docker compose -f docker/docker-compose.windows.yml ps --format "table {{.Service}}\t{{.Status}}"
    if ($ps -match "healthy" -or $ps -match "Up ") {
        Write-Host "    Services ready!" -ForegroundColor Green
        break
    }
    Start-Sleep 5
    $elapsed += 5
    Write-Host "    ...$elapsed/$maxWait sec" -NoNewline
    Write-Host "`r" -NoNewline
}
Write-Host ""

# Launch orchestrator
if (-not $NoTUI) {
    Write-Host "[6/6] Launching Ultron TUI..." -ForegroundColor Yellow
    Write-Host "    Press Ctrl+C to exit" -ForegroundColor Gray
    Write-Host ""
    hcs-orchestrator tui
} else {
    Write-Host "[6/6] Services running in background." -ForegroundColor Yellow
    Write-Host "    Access:" -ForegroundColor Cyan
    Write-Host "      TUI:        hcs-orchestrator tui" -ForegroundColor Cyan
    Write-Host "      CLI:        hcs-orchestrator run task.yaml" -ForegroundColor Cyan
    Write-Host "      gRPC:       localhost:50051" -ForegroundColor Cyan
    Write-Host "      HTTP API:   localhost:8080" -ForegroundColor Cyan
    Write-Host "      Metrics:    localhost:9090/metrics" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "   ULTRON READY - Happy Automating!  " -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan