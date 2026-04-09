//! Detección de frameworks usando IA
//!
//! Analiza archivos del proyecto para identificar el framework principal,
//! lenguaje de programación, patrones de arquitectura y configuraciones.

use crate::ai::client::{TaskType, consultar_ia};
use crate::config::{FrameworkDetection, SentinelConfig};
use crate::stats::SentinelStats;
use colored::*;

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Detecta el framework y sus reglas usando IA analizando los archivos del proyecto
pub fn detectar_framework_con_ia(
    project_path: &Path,
    config: &SentinelConfig,
    stats: Arc<Mutex<SentinelStats>>,
) -> anyhow::Result<FrameworkDetection> {
    println!("{}", "🤖 Detectando framework con IA...".magenta());

    let archivos = SentinelConfig::listar_archivos_raiz(project_path);
    let archivos_str = archivos.join("\n");

    // Leer automáticamente archivos clave para mejorar la detección
    let mut contenido_extra = String::new();

    // Intentar leer package.json (proyectos JS/TS)
    if let Ok(package_json) = fs::read_to_string(project_path.join("package.json")) {
        let primeras_lineas: String = package_json
            .lines()
            .take(50) // Primeras 50 líneas incluyen dependencies
            .collect::<Vec<_>>()
            .join("\n");
        contenido_extra.push_str(&format!(
            "\n\nCONTENIDO DE package.json (primeras 50 líneas):\n{}",
            primeras_lineas
        ));
    }

    // Intentar leer requirements.txt (proyectos Python)
    if let Ok(requirements) = fs::read_to_string(project_path.join("requirements.txt")) {
        let primeras_lineas: String = requirements.lines().take(30).collect::<Vec<_>>().join("\n");
        contenido_extra.push_str(&format!(
            "\n\nCONTENIDO DE requirements.txt:\n{}",
            primeras_lineas
        ));
    }

    // Intentar leer composer.json (proyectos PHP)
    if let Ok(composer_json) = fs::read_to_string(project_path.join("composer.json")) {
        let primeras_lineas: String = composer_json
            .lines()
            .take(40)
            .collect::<Vec<_>>()
            .join("\n");
        contenido_extra.push_str(&format!(
            "\n\nCONTENIDO DE composer.json:\n{}",
            primeras_lineas
        ));
    }

    let prompt_inicial = format!(
        "Eres un Experto en Arquitectura de Software. Tu tarea es identificar el \"Framework de Alto Nivel\" \
        que gobierna la arquitectura del proyecto.\n\n\
        CONTEXTO:\n\
        Archivos raíz: {}\
        {}\n\n\
        INSTRUCCIONES CRÍTICAS DE DIFERENCIACIÓN:\n\
        1. Framework vs Lenguaje: No respondas con el nombre del lenguaje (ej. TypeScript, Python). \
        Identifica el framework que dicta la estructura (ej. React, FastAPI, NestJS).\n\
        2. Jerarquía de Decisión:\n\
        - Si detectas 'react', el framework es \"React\" (aunque use Vite o Next, prioriza el ecosistema).\n\
        - Si detectas '@nestjs/core', el framework es \"NestJS\", no \"Node.js\".\n\
        - Si detectas 'actix-web' o 'axum' en un Cargo.toml, el framework es el nombre del crate.\n\
        3. Precisión en Monorepos: Si ves múltiples configuraciones, identifica la que define la ejecución principal.\n\n\
        RESPONDE EXCLUSIVAMENTE EN JSON:\n\
        {{\n\
        \"framework\": \"Nombre específico del framework (ej. React, Django, Axum)\",\n\
        \"code_language\": \"Lenguaje base (ej. typescript, rust, python)\",\n\
        \"rules\": [\"4 principios técnicos clave\"],\n\
        \"extensions\": [\"ts\", \"tsx\", \"js\", etc],\n\
        \"parent_patterns\": [\"sufijos de arquitectura\"],\n\
        \"test_patterns\": [\"rutas de tests con {{{{name}}}}\"]\n\
        }}\n\n\
        IMPORTANTE: Si no hay un framework claro, identifica la librería de entrada (entry-point) principal. \
        Prohibido responder con nombres genéricos como \"JavaScript/TypeScript\".",
        archivos_str, contenido_extra
    );

    // Primera consulta
    let respuesta = consultar_ia(
        prompt_inicial,
        &config.primary_model,
        Arc::clone(&stats),
        TaskType::Deep,
    )?;

    // Si la IA pide leer un archivo
    if respuesta.trim().starts_with("LEER:") {
        let archivo = respuesta.trim().replace("LEER:", "").trim().to_string();
        let archivo_path = project_path.join(&archivo);

        println!("   📄 IA solicita leer: {}", archivo.cyan());

        if let Ok(contenido) = fs::read_to_string(&archivo_path) {
            // Limitar contenido a primeras 100 líneas para no saturar
            let contenido_limitado: String =
                contenido.lines().take(100).collect::<Vec<_>>().join("\n");

            let prompt_con_contenido = format!(
                "Eres un Experto en Arquitectura de Software. Identifica el \"Framework de Alto Nivel\" del proyecto.\n\n\
                CONTEXTO:\n\
                Archivos raíz: {}\n\n\
                Contenido de '{}':\n{}\n\n\
                INSTRUCCIONES CRÍTICAS:\n\
                1. Framework vs Lenguaje: Identifica el framework que dicta la arquitectura, NO el lenguaje.\n\
                2. Jerarquía:\n\
                - 'react' en dependencies → Framework: \"React\"\n\
                - '@nestjs/core' → Framework: \"NestJS\"\n\
                - 'Django' → Framework: \"Django\"\n\
                3. Prohibido responder con nombres genéricos como \"TypeScript\" o \"JavaScript\".\n\n\
                RESPONDE EN JSON:\n\
                {{\n\
                \"framework\": \"Nombre específico del framework\",\n\
                \"code_language\": \"lenguaje base\",\n\
                \"rules\": [\"4 principios clave\"],\n\
                \"extensions\": [\"extensiones\"],\n\
                \"parent_patterns\": [\"sufijos o []\"],\n\
                \"test_patterns\": [\"rutas con {{{{name}}}}\"]\n\
                }}\n\n\
                IMPORTANTE: SOLO JSON, sin texto adicional.",
                archivos_str, archivo, contenido_limitado
            );

            let respuesta_final = consultar_ia(
                prompt_con_contenido,
                &config.primary_model,
                Arc::clone(&stats),
                TaskType::Deep,
            )?;

            return parsear_deteccion_framework(&respuesta_final);
        }
    }

    // Parsear respuesta JSON
    parsear_deteccion_framework(&respuesta)
}

/// Parsea la respuesta JSON de la IA con la detección del framework
fn parsear_deteccion_framework(respuesta: &str) -> anyhow::Result<FrameworkDetection> {
    println!("   🔍 [DEBUG-SENTINEL-V3] Procesando respuesta de IA (longitud: {})", respuesta.len());
    if respuesta.len() < 500 {
        println!("   🔍 [DEBUG-SENTINEL-V3] Contenido RAW: {}", respuesta.blue());
    }

    // 1. Limpiar posibles bloques de código markdown
    let clean_resp = respuesta
        .replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string();

    // 2. Intentar encontrar el inicio real del JSON
    let mut inicio_idx = clean_resp.find('{');
    while let Some(pos) = inicio_idx {
        let snippet = &clean_resp[pos..];
        if snippet.starts_with("{\"") || snippet.starts_with("{ \"") || snippet.starts_with("{\n") || snippet.starts_with("{\r") {
            inicio_idx = Some(pos);
            break;
        }
        // Buscar el siguiente '{'
        if pos + 1 < clean_resp.len() {
            inicio_idx = clean_resp[pos + 1..].find('{').map(|n| pos + 1 + n);
        } else {
            inicio_idx = None;
            break;
        }
    }

    let json_str = if let Some(inicio) = inicio_idx {
        if let Some(fin) = clean_resp.rfind('}') {
            &clean_resp[inicio..=fin]
        } else {
            &clean_resp
        }
    } else {
        &clean_resp
    };

    match serde_json::from_str::<FrameworkDetection>(json_str) {
        Ok(deteccion) => {
            println!("   ✅ Framework detectado: {}", deteccion.framework.green());
            Ok(deteccion)
        }
        Err(e) => {
            // Fallback si falla el parsing
            println!(
                "   ⚠️  Error al parsear respuesta de IA: {}",
                e.to_string().yellow()
            );
            println!("   ℹ️  JSON extraído: {}", json_str.dimmed());
            println!("   ℹ️  Usando configuración genérica. Edita .sentinelrc.toml después.");
            Ok(FrameworkDetection {
                framework: "Generic".to_string(),
                rules: vec![
                    "Clean Code principles".to_string(),
                    "SOLID design patterns".to_string(),
                    "Code maintainability".to_string(),
                    "Comprehensive testing".to_string(),
                ],
                extensions: vec!["js".to_string(), "ts".to_string()],
                code_language: "typescript".to_string(),
                parent_patterns: vec![],
                test_patterns: vec!["{name}.test.ts".to_string(), "{name}.spec.ts".to_string()],
            })
        }
    }
}

/// Obtiene el listado de modelos disponibles para cualquier proveedor
pub fn obtener_modelos_disponibles(
    provider: &str,
    api_url: &str,
    api_key: &str,
) -> anyhow::Result<Vec<String>> {
    let config = crate::config::ModelConfig {
        provider: provider.to_string(),
        url: api_url.to_string(),
        api_key: api_key.to_string(),
        name: String::new(),
    };
    crate::ai::providers::build_provider(&config).list_models()
}
