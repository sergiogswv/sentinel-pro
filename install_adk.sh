#!/bin/bash
# Sentinel ADK — Setup Script (Bash)
set -e

# Colores
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "\n${CYAN}[ Sentinel ADK Setup ]${NC}"

# 1. Verificar uv
if ! command -v uv &> /dev/null; then
    echo -e "${RED}Error: uv not found${NC}"
    exit 1
fi

# 2. Verificar directorio
if [ ! -d "sentinel_adk" ]; then
    echo -e "${RED}Error: sentinel_adk directory not found${NC}"
    exit 1
fi

# 3. Entorno virtual
if [ ! -d ".venv" ]; then
    echo -e "${YELLOW}Creating .venv...${NC}"
    uv venv > /dev/null
else
    echo -e ".venv already exists"
fi

# 4. Instalar dependencias
echo -e "${YELLOW}Installing dependencies...${NC}"
uv pip install -r sentinel_adk/requirements.txt > /dev/null

# 5. Archivo .env
if [ ! -f "sentinel_adk/.env" ]; then
    if [ -f "sentinel_adk/.env.example" ]; then
        echo -e "${YELLOW}Creating sentinel_adk/.env...${NC}"
        cp sentinel_adk/.env.example sentinel_adk/.env
    fi
fi

# 6. Verificar importación
echo -e "${YELLOW}Verifying module imports...${NC}"
PY_CODE="import sys; sys.path.insert(0, '.'); from sentinel_adk.main import app; print('Module imports correctly')"
if uv run python3 -c "$PY_CODE" 2>&1; then
    echo -e "${GREEN}Success: Module imports correctly${NC}"
else
    echo -e "${RED}Import Error${NC}"
    exit 1
fi

# 7. Verificar Core
if [ -f "target/release/sentinel" ] || [ -f "target/release/sentinel.exe" ]; then
    echo -e "${GREEN}Core executable found${NC}"
else
    echo -e "${YELLOW}Core executable not found (must build with cargo)${NC}"
fi

echo -e "\n${GREEN}Setup completed.${NC}"
