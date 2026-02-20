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

# Compilar e Instalar el proyecto globalmente
Show-Info "Instalando Sentinel Pro globalmente con cargo..."
try {
    # Usamos cargo install para que esté disponible en cualquier terminal automáticamente
    cargo install --path . --force
    Show-Success "Sentinel ha sido instalado en tu directorio de binarios de Rust (~\.cargo\bin)"
} catch {
    Show-Error "Falló la instalación vía cargo. Asegúrate de que no haya procesos de Sentinel abiertos."
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

# --- SECCIÓN QDRANT ---
Write-Host ""
$choice = Read-Host "❓ ¿Deseas instalar Qdrant (Vector Database) automáticamente? (s/n)"
if ($choice -eq 's' -or $choice -eq 'S') {
    Show-Info "Iniciando instalación de Qdrant..."
    
    $qdrantBaseDir = "$installDir\qdrant"
    if (-not (Test-Path $qdrantBaseDir)) {
        New-Item -ItemType Directory -Path $qdrantBaseDir -Force | Out-Null
    }

    try {
        Show-Info "Obteniendo última versión de Qdrant desde GitHub..."
        $githubApiUrl = "https://api.github.com/repos/qdrant/qdrant/releases/latest"
        $latestRelease = Invoke-RestMethod -Uri $githubApiUrl -UseBasicParsing
        $version = $latestRelease.tag_name
        
        $downloadUrl = "https://github.com/qdrant/qdrant/releases/download/$version/qdrant-x86_64-pc-windows-msvc.zip"
        $zipFile = "$qdrantBaseDir\qdrant.zip"
        
        Show-Info "Descargando Qdrant $version..."
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipFile
        
        Show-Info "Extrayendo archivos..."
        Expand-Archive -Path $zipFile -DestinationPath $qdrantBaseDir -Force
        Remove-Item $zipFile
        
        # Crear script de inicio rápido para Qdrant
        $qdrantRunScript = "$qdrantBaseDir\run-qdrant.ps1"
        $runContent = @"
# Script para iniciar Qdrant localmente
Set-Location -Path "`$PSScriptRoot"
.\qdrant.exe
"@
        Set-Content -Path $qdrantRunScript -Value $runContent -Encoding UTF8
        
        Show-Success "Qdrant instalado en $qdrantBaseDir"
        Show-Info "Puedes iniciarlo ejecutando: $qdrantRunScript"
        
        $installQdrantPath = $true
    } catch {
        Show-Error "No se pudo instalar Qdrant automáticamente: $($_.Exception.Message)"
    }
} else {
    Show-Info "Omitiendo instalación automática de Qdrant. Puedes instalarlo manualmente después."
    $installQdrantPath = $false
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
