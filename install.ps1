# Sentinel Rust - Script de instalación para Windows PowerShell
# Versión: 4.5.0

# Configurar para detener en errores
$ErrorActionPreference = "Stop"

# Función para mostrar banner
function Show-Banner {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Blue
    Write-Host "║                                                           ║" -ForegroundColor Blue
    Write-Host "║              🛡️  SENTINEL RUST INSTALLER 🛡️               ║" -ForegroundColor Blue
    Write-Host "║                                                           ║" -ForegroundColor Blue
    Write-Host "║           AI-Powered Code Quality Guardian                ║" -ForegroundColor Blue
    Write-Host "║                    Version 4.5.0                          ║" -ForegroundColor Blue
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

# Compilar el proyecto
Show-Info "Compilando Sentinel Rust..."
try {
    cargo build --release
    Show-Success "Compilación exitosa"
} catch {
    Show-Error "Falló la compilación del proyecto"
}

# Crear directorio de instalación
$installDir = "$env:USERPROFILE\.sentinel-pro"
Show-Info "Creando directorio de instalación en $installDir..."
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Copiar el binario
Show-Info "Instalando binario..."
$binarySource = "target\release\sentinel-pro.exe"
$binaryDest = "$installDir\sentinel-pro.exe"

if (-not (Test-Path $binarySource)) {
    Show-Error "No se encontró el binario compilado en $binarySource"
}

Copy-Item $binarySource $binaryDest -Force
Show-Success "Binario instalado en $binaryDest"

# Agregar al PATH del usuario si no está
Show-Info "Verificando PATH del sistema..."
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Show-Info "Agregando Sentinel al PATH del usuario..."
    [Environment]::SetEnvironmentVariable(
        "Path",
        "$userPath;$installDir",
        "User"
    )
    Show-Success "PATH actualizado. Por favor reinicia tu terminal para aplicar los cambios."
} else {
    Show-Info "Sentinel ya está en el PATH"
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
Write-Host "4. Ejecuta Sentinel Pro en tu proyecto:" -ForegroundColor White
Write-Host "   cd C:\ruta\a\tu\proyecto" -ForegroundColor Yellow
Write-Host "   sentinel-pro" -ForegroundColor Yellow
Write-Host ""
Write-Host "🎉 ¡Disfruta de Sentinel Pro!" -ForegroundColor Green
Write-Host ""
