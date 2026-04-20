# Sentinel ADK Setup Script
$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

Write-Host "`n[ Sentinel ADK Setup ]" -ForegroundColor Cyan

# 1. Check uv
if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "Error: uv not found" -ForegroundColor Red
    exit 1
}

# 2. Check directory
if (-not (Test-Path "sentinel_adk")) {
    Write-Host "Error: sentinel_adk directory not found" -ForegroundColor Red
    exit 1
}

# 3. Virtual environment
if (-not (Test-Path ".venv")) {
    Write-Host "Creating .venv..." -ForegroundColor Yellow
    uv venv | Out-Null
} else {
    Write-Host ".venv already exists"
}

# 4. Install dependencies
Write-Host "Installing dependencies..." -ForegroundColor Yellow
uv pip install -r sentinel_adk/requirements.txt | Out-Null

# 5. .env file
if (-not (Test-Path "sentinel_adk/.env")) {
    if (Test-Path "sentinel_adk/.env.example") {
        Write-Host "Creating sentinel_adk/.env..." -ForegroundColor Yellow
        Copy-Item "sentinel_adk/.env.example" "sentinel_adk/.env"
    }
}

# 6. Verify import
Write-Host "Verifying module imports..." -ForegroundColor Yellow
$pyCode = "import sys; sys.path.insert(0, '.'); from sentinel_adk.main import app; print('Module imports correctly')"
$result = uv run python -c $pyCode 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "Success: $result" -ForegroundColor Green
} else {
    Write-Host "Import Error: $result" -ForegroundColor Red
    exit 1
}

# 7. Check Core
if (Test-Path "target/release/sentinel.exe") {
    Write-Host "Core executable found" -ForegroundColor Green
} else {
    Write-Host "Core executable not found (must build with cargo)" -ForegroundColor Yellow
}

Write-Host "`nSetup completed." -ForegroundColor Green
