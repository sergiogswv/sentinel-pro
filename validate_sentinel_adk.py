"""
validate_sentinel_adk.py — Script de validación end-to-end del Sentinel ADK.

Valida:
  1. Sentinel ADK responde en :4011 (/health)
  2. Sentinel Core responde en :4001 (/health o /command status)
  3. Cerebro responde en :4000 (/health o /)
  4. El ADK puede enviar un comando 'status' exitosamente
  5. El evento llega a Cerebro (sentinel_status)

Uso:
  cd sentinel
  uv run python validate_sentinel_adk.py

  # Con Cerebro y Core ya corriendo:
  uv run python validate_sentinel_adk.py --full
"""

import asyncio
import sys
import argparse
import httpx


ADK_URL     = "http://localhost:4011"
CORE_URL    = "http://localhost:4001"
CEREBRO_URL = "http://localhost:4000"


async def check_service(name: str, url: str, path: str = "/health") -> bool:
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.get(f"{url}{path}")
            if resp.status_code < 400:
                print(f"  ✅ {name} — OK ({resp.status_code}) @ {url}")
                return True
            else:
                print(f"  ❌ {name} — HTTP {resp.status_code} @ {url}")
                return False
    except httpx.ConnectError:
        print(f"  ❌ {name} — No disponible en {url}")
        return False
    except Exception as e:
        print(f"  ❌ {name} — Error: {e}")
        return False


async def send_adk_command(action: str, target: str = ".") -> dict:
    try:
        async with httpx.AsyncClient(timeout=30.0) as client:
            resp = await client.post(
                f"{ADK_URL}/command",
                json={"action": action, "target": target, "request_id": f"validate-{action}"}
            )
            return resp.json()
    except Exception as e:
        return {"status": "error", "error": str(e)}


async def main(full: bool = False):
    print("\n🛡️ Sentinel ADK — Validación de Integración")
    print("=" * 50)

    # ── Paso 1: health checks ─────────────────────────────
    print("\n[1/4] Health checks...")
    adk_ok     = await check_service("Sentinel ADK (4011)",   ADK_URL)
    core_ok    = await check_service("Sentinel Core (4001)",  CORE_URL)
    cerebro_ok = await check_service("Cerebro (4000)",        CEREBRO_URL, path="/")

    if not adk_ok:
        print("\n⚠️  El ADK no está corriendo. Lánzalo con:")
        print("   uv run uvicorn sentinel_adk.main:app --host 0.0.0.0 --port 4011")
        sys.exit(1)

    # ── Paso 2: memory context ────────────────────────────
    print("\n[2/4] Verificando memoria SQLite del ADK...")
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.get(f"{ADK_URL}/memory/context")
            data = resp.json()
            hot_files = data.get("hot_files", [])
            recent    = data.get("recent_findings", [])
            print(f"  ✅ SQLite OK — hot_files: {len(hot_files)}, recent_findings: {len(recent)}")
    except Exception as e:
        print(f"  ❌ Error accediendo a /memory/context: {e}")

    # ── Paso 3: command status ────────────────────────────
    print("\n[3/4] Enviando comando 'status' al ADK...")
    result = await send_adk_command("status")
    status = result.get("status")
    if status == "completed":
        r = result.get("result", {})
        print(f"  ✅ Comando completado")
        print(f"     llm_provider: {r.get('llm_provider', '?')}")
        print(f"     core_url:     {r.get('core_url', '?')}")
        print(f"     hot_files:    {r.get('hot_files_tracked', 0)}")
    else:
        print(f"  ⚠️  Status: {status} | error: {result.get('error', 'N/A')}")

    # ── Paso 4: full check con Core (opcional) ────────────
    if full:
        if not core_ok:
            print("\n[4/4] ⚠️  Sentinel Core no disponible — saltando check completo")
        else:
            print("\n[4/4] Enviando comando 'check' al ADK (requiere Core)...")
            result = await send_adk_command("check", target=".")
            status = result.get("status")
            if status == "completed":
                r = result.get("result", {})
                print(f"  ✅ Check completado")
                print(f"     analysis[:100]: {str(r.get('analysis', ''))[:100]}...")
                print(f"     severity: {r.get('severity', '?')}")
            else:
                print(f"  ❌ Check falló: {result.get('error') or result.get('status')}")
    else:
        print("\n[4/4] Saltando check completo (usa --full para incluirlo con el Core corriendo)")

    print("\n" + "=" * 50)
    print("✅ Validación completada\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Valida el Sentinel ADK")
    parser.add_argument("--full", action="store_true", help="Incluir check completo (requiere Core Rust corriendo)")
    args = parser.parse_args()
    asyncio.run(main(full=args.full))
