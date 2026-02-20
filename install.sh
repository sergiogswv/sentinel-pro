#!/bin/bash
# Sentinel Rust - Script de instalación para Linux/macOS
# Versión: 5.0.0-pro

set -e

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}"
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║              🛡️  SENTINEL INSTALLER 🛡️                    ║"
echo "║                                                           ║"
echo "║           AI-Powered Code Quality Guardian                ║"
echo "║                    Version 5.0.0-pro                      ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Función para mostrar errores
error() {
    echo -e "${RED}❌ Error: $1${NC}"
    exit 1
}

# Función para mostrar éxito
success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Función para mostrar información
info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# Verificar si Rust está instalado
info "Verificando instalación de Rust..."
if ! command -v cargo &> /dev/null; then
    error "Rust no está instalado. Por favor instala Rust desde https://rustup.rs/"
fi
success "Rust encontrado: $(rustc --version)"

# Verificar versión de Rust (requiere edition 2024)
RUST_VERSION=$(rustc --version | awk '{print $2}')
info "Versión de Rust: $RUST_VERSION"

# Compilar e instalar globalmente vía Cargo
info "Compilando e instalando Sentinel Pro globalmente con cargo..."
cargo install --path . --force || error "Falló la instalación vía cargo"
success "Sentinel instalado en su directorio de binarios de Rust (~/.cargo/bin)"

# Crear directorio de recursos de Sentinel
INSTALL_DIR="$HOME/.sentinel-pro"
info "Configurando directorio de recursos en $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"

# Agregar ~/.cargo/bin al PATH si no está (solo una vez)
SHELL_RC=""
if [ -f "$HOME/.bashrc" ]; then SHELL_RC="$HOME/.bashrc"; elif [ -f "$HOME/.zshrc" ]; then SHELL_RC="$HOME/.zshrc"; fi

if [ -n "$SHELL_RC" ] && ! grep -q ".cargo/bin" "$SHELL_RC"; then
    info "Asegurando que .cargo/bin esté en el PATH en $SHELL_RC..."
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$SHELL_RC"
    success "PATH actualizado en $SHELL_RC"
fi

# Eliminar alias antiguos que apunten a versiones anteriores
if [ -n "$SHELL_RC" ] && grep -q "alias sentinel=" "$SHELL_RC"; then
    info "Eliminando alias antiguo de Sentinel en $SHELL_RC..."
    sed -i.bak '/alias sentinel=/d' "$SHELL_RC"
    success "Alias antiguo eliminado de $SHELL_RC"
fi

# Crear directorio para la Knowledge Base
CONFIG_DIR="$HOME/.sentinel-pro"
mkdir -p "$CONFIG_DIR"

# Crear archivo de configuración de ejemplo si no existe
CONFIG_FILE="$CONFIG_DIR/sentinel.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    info "Creando archivo de configuración de ejemplo..."
    cat > "$CONFIG_FILE" << 'EOF'
# Configuración de Sentinel Pro
[sentinel]
framework = "Rust"
code_language = "rust"

[ai]
api_key = "tu-api-key-aqui"
model = "claude-3-5-sonnet"
EOF
    success "Archivo de configuración creado en $CONFIG_FILE"
fi

echo -e "${GREEN}"
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║          ✨ INSTALACIÓN COMPLETADA EXITOSAMENTE ✨        ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo -e "${BLUE}📋 Próximos pasos:${NC}"
echo ""
echo "1. Recarga tu terminal o ejecuta:"
echo -e "   ${YELLOW}source $SHELL_RC${NC}"
echo -e "   (Nota: Si la terminal sigue ejecutando una versión antigua, usa: ${YELLOW}hash -r${NC})"
echo ""
echo "2. Configura tu API key en:"
echo -e "   ${YELLOW}$CONFIG_FILE${NC}"
echo ""
echo "3. Ejecuta Sentinel en tu proyecto:"
echo -e "   ${YELLOW}sentinel${NC}"
echo ""
echo -e "${GREEN}🎉 ¡Disfruta de Sentinel Pro!${NC}"
echo ""
