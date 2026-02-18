#!/bin/bash
# Sentinel Rust - Script de instalación para Linux/macOS
# Versión: 4.5.0

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
echo "║              🛡️  SENTINEL RUST INSTALLER 🛡️               ║"
echo "║                                                           ║"
echo "║           AI-Powered Code Quality Guardian                ║"
echo "║                    Version 4.5.0                          ║"
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

# Compilar el proyecto
info "Compilando Sentinel Pro..."
cargo build --release || error "Falló la compilación del proyecto"
success "Compilación exitosa"

# Crear directorio de instalación
# Preferimos ~/.local/bin si existe, sino ~/.sentinel-pro
INSTALL_DIR="$HOME/.sentinel-pro"
BIN_NAME="sentinel-pro"

if [ -d "$HOME/.local/bin" ] && [[ ":$PATH:" == *":$HOME/.local/bin:"* ]]; then
    INSTALL_DIR="$HOME/.local/bin"
    info "Detectado ~/.local/bin en el PATH. Instalando allí..."
else
    info "Creando directorio de instalación en $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
fi

# Copiar el binario
info "Instalando binario en $INSTALL_DIR..."
cp target/release/sentinel-pro "$INSTALL_DIR/$BIN_NAME" || error "Falló la copia del binario"
chmod +x "$INSTALL_DIR/$BIN_NAME"
success "Binario instalado en $INSTALL_DIR/$BIN_NAME"

# Agregar al PATH si no está
SHELL_RC=""
if [ -f "$HOME/.bashrc" ]; then
    SHELL_RC="$HOME/.bashrc"
elif [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
fi

if [ -n "$SHELL_RC" ]; then
    if ! grep -q "sentinel-pro" "$SHELL_RC"; then
        info "Agregando Sentinel Pro al PATH en $SHELL_RC..."
        echo "" >> "$SHELL_RC"
        echo "# Sentinel Pro" >> "$SHELL_RC"
        echo "export PATH=\"\$HOME/.sentinel-pro:\$PATH\"" >> "$SHELL_RC"
        success "PATH actualizado. Por favor ejecuta: source $SHELL_RC"
    else
        info "Sentinel Pro ya está en el PATH"
    fi
fi

# Crear archivo de configuración de ejemplo si no existe
CONFIG_FILE="$HOME/.sentinel-rust/sentinel.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    info "Creando archivo de configuración de ejemplo..."
    cat > "$CONFIG_FILE" << 'EOF'
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
EOF
    success "Archivo de configuración creado en $CONFIG_FILE"
fi

echo ""
echo -e "${GREEN}"
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║          ✨ INSTALACIÓN COMPLETADA EXITOSAMENTE ✨         ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""
echo -e "${BLUE}📋 Próximos pasos:${NC}"
echo ""
echo "1. Recarga tu shell:"
echo -e "   ${YELLOW}source $SHELL_RC${NC}"
echo ""
echo "2. Configura tu API key de Claude:"
echo -e "   ${YELLOW}Edita: $CONFIG_FILE${NC}"
echo ""
echo "3. Copia sentinel.toml a tu proyecto:"
echo -e "   ${YELLOW}cp $CONFIG_FILE /ruta/a/tu/proyecto/${NC}"
echo ""
echo "4. Ejecuta Sentinel en tu proyecto:"
echo -e "   ${YELLOW}cd /ruta/a/tu/proyecto${NC}"
echo -e "   ${YELLOW}sentinel${NC}"
echo ""
echo -e "${GREEN}🎉 ¡Disfruta de Sentinel Rust!${NC}"
echo ""
