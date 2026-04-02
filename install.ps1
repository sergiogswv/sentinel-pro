# Sentinel Rust - Script de instalación para Windows PowerShell
# Versión: 5.0.0-pro

# Configurar para detener en errores
$ErrorActionPreference = "Stop"

# Función para mostrar banner
function Show-Banner {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Blue
    Write-Host "║                                                           ║" -ForegroundColor Blue
    Write-Host "║              🛡️  SENTINEL INSTALLER 🛡️                   ║" -ForegroundColor Blue
    Write-Host "║                                                           ║" -ForegroundColor Blue
    Write-Host "║           AI-Powered Code Quality Guardian                ║" -ForegroundColor Blue
    Write-Host "║                    Version 5.0.0-pro                      ║" -ForegroundColor Blue
    Write-Host "║                                                           ║" -ForegroundColor Blue
    Write-Host "╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Blue
    Write-Host ""
}

# Función para mostrar errores
function Show-Error {
    param([string]$Message)
    Write-Host "❌ Error: $Message" -ForegroundColor Red
    exit 1
}

# Función para mostrar éxito
function Show-Success {
    param([string]$Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

# Función para mostrar información
function Show-Info {
    param([string]$Message)
    Write-Host "ℹ️  $Message" -ForegroundColor Yellow
}

# Mostrar banner
Show-Banner

# Verificar si Rust está instalado
Show-Info "Verificando instalación de Rust..."
try {
    $rustVersion = cargo --version
    Show-Success "Rust encontrado: $rustVersion"
} catch {
    Show-Error "Rust no está instalado. Por favor instala Rust desde https://rustup.rs/"
}

# Verificar versión de rustc
$rustcVersion = rustc --version
Show-Info "Versión de Rust: $rustcVersion"

# Compilar e Instalar el proyecto
Show-Info "Compilando Sentinel Pro en modo release..."
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Show-Error "La compilación falló. Revisa los errores arriba."
    }
    Show-Success "Compilación exitosa."
} catch {
    Show-Error "No se pudo ejecutar cargo build. ¿Está Rust instalado?"
}

# Recolectar todas las ubicaciones donde instalar
$destinos = @()

# 1. ~/bin (instalación propia)
$binPath = "$env:USERPROFILE\bin"
if (!(Test-Path $binPath)) {
    Show-Info "Creando carpeta $binPath..."
    New-Item -ItemType Directory -Path $binPath | Out-Null
}
$destinos += "$binPath\sentinel.exe"

# 2. ~/.cargo/bin (standard Rust location)
$cargoBin = "$env:USERPROFILE\.cargo\bin\sentinel.exe"
# Siempre intentamos actualizar este también si existe o si queremos que esté allí
$destinos += $cargoBin

# Copiar a cada destino
$copiasFallidas = @()
foreach ($destino in $destinos) {
    $timestampAntes = $null
    if (Test-Path $destino) {
        $timestampAntes = (Get-Item $destino).LastWriteTime
    }

    Show-Info "Instalando en: $destino..."
    try {
        Copy-Item "target\release\sentinel.exe" -Destination $destino -Force -ErrorAction Stop

        # Verificar que la copia realmente cambió el archivo
        $timestampDespues = (Get-Item $destino).LastWriteTime
        if ($timestampAntes -and $timestampAntes -eq $timestampDespues) {
            Write-Host "  ⚠️ ADVERTENCIA: El archivo no cambió. Puede estar bloqueado." -ForegroundColor Yellow
            $copiasFallidas += $destino
        } else {
            Show-Success "  OK"
        }
    } catch {
        Write-Host "  ❌ ERROR: $_" -ForegroundColor Red
        Write-Host "  El archivo puede estar en uso por Sentinel Monitor o VS Code. Ciérralos y reintenta." -ForegroundColor Yellow
        $copiasFallidas += $destino
    }
}

if ($copiasFallidas.Count -gt 0) {
    Write-Host ""
    Write-Host "No se pudieron actualizar todos los binarios:" -ForegroundColor Red
    $copiasFallidas | ForEach-Object { Write-Host "  $_" -ForegroundColor White }
    Write-Host "Cierra todas las terminales y aplicaciones que usen Sentinel y vuelve a ejecutar el script." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

# Crear directorio de casa de Sentinel para recursos (Qdrant, Modelos, etc.)
$installDir = "$env:USERPROFILE\.sentinel-pro"
Show-Info "Configurando directorio de recursos en $installDir..."
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Verificar PATH para el folder de Cargo (standard)
$cargoBin = "$env:USERPROFILE\.cargo\bin"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")

if ($userPath -notlike "*$cargoBin*") {
    Show-Info "Agregando .cargo\bin al PATH para que 'sentinel' funcione en cualquier parte..."
    $newPath = if ($userPath.EndsWith(';')) { "$userPath$cargoBin" } else { "$userPath;$cargoBin" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path += ";$cargoBin"
    Show-Success "PATH de Cargo actualizado."
} else {
    Show-Info "Directorio de binarios ya está en el PATH."
}

# Crear archivo de configuración de ejemplo si no existe
$configFile = "$installDir\sentinel.toml"
if (-not (Test-Path $configFile)) {
    Show-Info "Creando archivo de configuración de ejemplo..."
    
    $configContent = @"
# Configuración de Sentinel Rust
# Copia este archivo a la raíz de tu proyecto y personalízalo

[sentinel]
framework = "Rust"
code_language = "rust"

# Reglas de arquitectura específicas
architecture_rules = [
    "Usa Result<T, E> para manejo de errores",
    "Evita unwrap() en código de producción",
    "Implementa traits apropiados (Debug, Clone, etc.)",
    "Usa ownership correctamente para evitar clones innecesarios",
    "Documenta funciones públicas con ///"
]

# Configuración de la API de IA
[ai]
api_key = "tu-api-key-aqui"
model = "claude-3-5-sonnet-20241022"
max_tokens = 4000
"@
    
    Set-Content -Path $configFile -Value $configContent -Encoding UTF8
    Show-Success "Archivo de configuración creado en $configFile"
}


# Mostrar mensaje de éxito
Write-Host ""
Write-Host "╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║                                                           ║" -ForegroundColor Green
Write-Host "║          ✨ INSTALACIÓN COMPLETADA EXITOSAMENTE ✨         ║" -ForegroundColor Green
Write-Host "║                                                           ║" -ForegroundColor Green
Write-Host "╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""

Write-Host "📋 Próximos pasos:" -ForegroundColor Blue
Write-Host ""
Write-Host "1. Reinicia tu terminal para aplicar los cambios al PATH" -ForegroundColor White
Write-Host ""
Write-Host "2. Configura tu API key de Claude:" -ForegroundColor White
Write-Host "   Edita: $configFile" -ForegroundColor Yellow
Write-Host ""
Write-Host "3. Copia sentinel.toml a tu proyecto:" -ForegroundColor White
Write-Host "   Copy-Item $configFile C:\ruta\a\tu\proyecto\" -ForegroundColor Yellow
Write-Host ""
Write-Host "4. Ejecuta Sentinel en tu proyecto:" -ForegroundColor White
Write-Host "   cd C:\ruta\a\tu\proyecto" -ForegroundColor Yellow
Write-Host "   sentinel" -ForegroundColor Yellow
Write-Host ""
Write-Host "🎉 ¡Disfruta de Sentinel Pro!" -ForegroundColor Green
Write-Host ""

# ─── INSTALACIÓN DEL ADK DE SENTINEL (Python) ────────────────────────────────
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  Instalando Sentinel ADK (Python Sidecar con IA)" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Verificar si Python está instalado
Show-Info "Verificando instalación de Python..."
try {
    $pythonVersion = python --version 2&1
    Show-Success "Python encontrado: $pythonVersion"
} catch {
    Show-Error "Python no está instalado. Por favor instala Python 3.9+ desde https://python.org/"
}

# Crear directorio del proyecto
$projectDir = $PSScriptRoot
if (-not $projectDir) {
    $projectDir = Get-Location
}

# Crear virtualenv si no existe
$venvPath = Join-Path $projectDir ".venv"
if (-not (Test-Path $venvPath)) {
    Show-Info "Creando virtualenv en $venvPath..."
    try {
        python -m venv .venv
        Show-Success "Virtualenv creado exitosamente."
    } catch {
        Show-Error "No se pudo crear el virtualenv. ¿Está Python instalado correctamente?"
    }
} else {
    Show-Info "Virtualenv ya existe en $venvPath"
}

# Instalar dependencias del ADK
Show-Info "Instalando dependencias del Sentinel ADK..."
try {
    & (Join-Path $venvPath "Scripts\Activate.ps1")
    pip install -r sentinel_adk/requirements.txt
    if ($LASTEXITCODE -eq 0) {
        Show-Success "Dependencias del ADK instaladas correctamente."
    } else {
        Show-Error "Error instalando dependencias del ADK."
    }
} catch {
    Show-Error "No se pudieron instalar las dependencias del ADK: $_"
}

# Crear archivo .env de ejemplo si no existe
$envFile = Join-Path $projectDir "sentinel_adk\.env"
$envExample = Join-Path $projectDir "sentinel_adk\.env.example"

if (-not (Test-Path $envFile) -and (Test-Path $envExample)) {
    Show-Info "Creando archivo de configuración .env..."
    Copy-Item $envExample $envFile
    Show-Success "Archivo .env creado. Por favor configura tus API keys."
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host "  ✨ INSTALACIÓN DEL ADK COMPLETADA ✨" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "Para configurar el ADK:" -ForegroundColor White
Write-Host "  1. Edita: sentinel_adk\.env" -ForegroundColor Yellow
Write-Host "  2. Agrega tu GOOGLE_API_KEY (obtenla en https://makersuite.google.com/app/apikey)" -ForegroundColor Yellow
Write-Host ""
