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
echo "║              🛡️  SENTINEL INSTALLER 🛡️                   ║"
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

[knowledge_base]
vector_db_url = "http://127.0.0.1:6334"
index_on_start = true

[ai]
api_key = "tu-api-key-aqui"
model = "claude-3-5-sonnet"
EOF
    success "Archivo de configuración creado en $CONFIG_FILE"
fi

# --- SECCIÓN QDRANT (Linux/macOS) ---
echo ""
read -p "❓ ¿Deseas instalar Qdrant (Vector Database) automáticamente? (s/n): " choice
if [[ "$choice" == "s" || "$choice" == "S" ]]; then
    info "Iniciando instalación de Qdrant..."
    QDRANT_DIR="$INSTALL_DIR/qdrant"
    mkdir -p "$QDRANT_DIR"
    
    # Detectar arquitectura y OS
    OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH_TYPE=$(uname -m)
    
    if [[ "$OS_TYPE" == "darwin" ]]; then
        DOWNLOAD_NAME="qdrant-x86_64-apple-darwin.tar.gz" # Default para Mac
        if [[ "$ARCH_TYPE" == "arm64" ]]; then DOWNLOAD_NAME="qdrant-aarch64-apple-darwin.tar.gz"; fi
    else
        DOWNLOAD_NAME="qdrant-x86_64-unknown-linux-gnu.tar.gz"
        if [[ "$ARCH_TYPE" == "aarch64" ]]; then DOWNLOAD_NAME="qdrant-aarch64-unknown-linux-gnu.tar.gz"; fi
    fi

    info "Obteniendo última versión desde GitHub..."
    LATEST_TAG=$(curl -s https://api.github.com/repos/qdrant/qdrant/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    info "Descargando Qdrant $LATEST_TAG..."
    curl -L "https://github.com/qdrant/qdrant/releases/download/$LATEST_TAG/$DOWNLOAD_NAME" -o "$QDRANT_DIR/qdrant.tar.gz"
    
    info "Extrayendo..."
    tar -xzf "$QDRANT_DIR/qdrant.tar.gz" -C "$QDRANT_DIR"
    rm "$QDRANT_DIR/qdrant.tar.gz"
    chmod +x "$QDRANT_DIR/qdrant"
    
    # Crear script de inicio
    cat > "$QDRANT_DIR/run-qdrant.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
./qdrant
EOF
    chmod +x "$QDRANT_DIR/run-qdrant.sh"
    
    success "Qdrant instalado en $QDRANT_DIR"
    info "Puedes iniciarlo manualmente con: $QDRANT_DIR/run-qdrant.sh"
fi

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
echo "1. Recarga tu terminal o ejecuta:"
echo -e "   ${YELLOW}source $SHELL_RC${NC}"
echo ""
echo "2. Configura tu API key en:"
echo -e "   ${YELLOW}$CONFIG_FILE${NC}"
echo ""
echo "3. Ejecuta Sentinel en tu proyecto:"
echo -e "   ${YELLOW}sentinel${NC}"
echo ""
echo -e "${GREEN}🎉 ¡Disfruta de Sentinel Pro!${NC}"
echo ""
