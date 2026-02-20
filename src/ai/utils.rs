//! Utilidades para procesamiento de respuestas de IA
//!
//! Funciones auxiliares para extraer y procesar bloques de código
//! desde respuestas de IA formateadas en markdown.

/// Elimina bloques de código de una respuesta de IA.
///
/// Busca y elimina el contenido entre delimitadores \`\`\`...\`\`\`,
/// dejando solo el texto explicativo. Útil para mostrar consejos
/// al usuario sin el código sugerido.
///
/// # Argumentos
///
/// * `texto` - Respuesta completa de IA con posibles bloques de código
///
/// # Retorna
///
/// Texto sin bloques de código, con marcadores indicando dónde estaban.
///
/// # Ejemplo
///
/// ```
/// let respuesta = "Aquí hay un problema:\n```rust\nfn main() {}\n```\nCorrige esto.";
/// let sin_codigo = eliminar_bloques_codigo(respuesta);
/// // Retorna: "Aquí hay un problema:\n[... Código guardado en .suggested ...]\nCorrige esto."
/// ```
pub fn eliminar_bloques_codigo(texto: &str) -> String {
    let mut resultado = String::new();
    let mut en_bloque = false;

    for linea in texto.lines() {
        if linea.trim().starts_with("```") {
            en_bloque = !en_bloque;
            if !en_bloque {
                resultado.push_str("\n[... Código guardado en .suggested ...]\n");
            }
            continue;
        }
        if !en_bloque {
            resultado.push_str(linea);
            resultado.push('\n');
        }
    }
    resultado.trim().to_string()
}

/// Extrae bloques de código de una respuesta de IA.
///
/// Busca y extrae el contenido entre delimitadores de markdown \`\`\`lenguaje...\`\`\`.
/// Soporta múltiples lenguajes comunes. Si no encuentra un bloque delimitado,
/// devuelve el texto completo.
///
/// # Argumentos
///
/// * `texto` - Respuesta completa de IA
///
/// # Retorna
///
/// Código extraído (sin delimitadores) o el texto original si no hay bloques.
///
/// # Lenguajes soportados
///
/// typescript, javascript, python, php, go, rust, java, jsx, tsx, code
///
/// # Ejemplo
///
/// ```
/// let respuesta = "Aquí está el código:\n```rust\nfn main() {\n    println!(\"Hola\");\n}\n```";
/// let codigo = extraer_codigo(respuesta);
/// // Retorna: "fn main() {\n    println!(\"Hola\");\n}"
/// ```
/// Extrae el bloque de código de un texto (markdown block ```).
/// Retorna None si no se encuentra ningún bloque.
pub fn extraer_codigo_opcional(texto: &str) -> Option<String> {
    let bloques = extraer_todos_bloques(texto);
    bloques.first().map(|(_, codigo)| codigo.clone())
}

/// Versión compatible con fallback a cadena vacía.
/// IMPORTANTE: Si no hay bloques, ya NO devuelve el texto completo para evitar romper archivos.
pub fn extraer_codigo(texto: &str) -> String {
    extraer_codigo_opcional(texto).unwrap_or_default()
}

/// Extrae TODOS los bloques de código de una respuesta de IA.
///
/// Retorna un Vec de tuplas (Option<ruta>, codigo) donde ruta es el comentario
/// de la primera línea si empieza por `//` o `#`.
pub fn extraer_todos_bloques(texto: &str) -> Vec<(Option<String>, String)> {
    let mut result = Vec::new();
    let mut in_block = false;
    let mut current = String::new();

    for line in texto.lines() {
        if line.trim().starts_with("```") {
            if in_block {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    let mut lines = trimmed.lines();
                    let first = lines.next().unwrap_or("").trim();
                    let path = if first.starts_with("//") || first.starts_with('#') {
                        let raw = first.trim_start_matches(|c| c == '/' || c == '#' || c == ' ').trim();
                        if raw.contains('.') {
                            Some(raw.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    result.push((path, trimmed));
                }
                current.clear();
                in_block = false;
            } else {
                in_block = true;
            }
        } else if in_block {
            current.push_str(line);
            current.push('\n');
        }
    }

    // Auto-cerrar bloque si terminó abruptamente (común en respuestas truncadas)
    if in_block && !current.trim().is_empty() {
        let trimmed = current.trim().to_string();
        let mut lines = trimmed.lines();
        let first = lines.next().unwrap_or("").trim();
        let path = if first.starts_with("//") || first.starts_with('#') {
            let raw = first.trim_start_matches(|c| c == '/' || c == '#' || c == ' ').trim();
            if raw.contains('.') { Some(raw.to_string()) } else { None }
        } else { None };
        result.push((path, trimmed));
    }

    result
}


/// Extrae un bloque JSON de una respuesta de IA.
pub fn extraer_json(texto: &str) -> String {
    // Primero intentar buscar bloque markdown ```json
    let bloques = extraer_todos_bloques(texto);
    for (_, code) in bloques {
        if code.trim_start().starts_with('{') || code.trim_start().starts_with('[') {
            return code;
        }
    }

    // Si no hay bloques, buscar patrón { ... } o [ ... ]
    if let Some(start) = texto.find('{') {
        if let Some(end) = texto.rfind('}') {
            return texto[start..=end].trim().to_string();
        }
    }

    if let Some(start) = texto.find('[') {
        if let Some(end) = texto.rfind(']') {
            return texto[start..=end].trim().to_string();
        }
    }

    texto.to_string()
}

/// Extrae un bloque JSON especializado en sugerencias de revisión.
/// Busca bloques que contengan campos clave como "impact" o "title".
pub fn extraer_json_sugerencias(texto: &str) -> String {
    let bloques = extraer_todos_bloques(texto);
    for (_, code) in bloques {
        if code.contains("\"impact\"") || code.contains("\"title\"") {
            return code;
        }
    }
    extraer_json(texto)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraer_codigo_typescript() {
        let texto = "Aquí está:\n```typescript\nconst x = 1;\n```\nEso es todo.";
        assert_eq!(extraer_codigo(texto), "const x = 1;");
    }

    #[test]
    fn test_extraer_codigo_sin_lenguaje() {
        let texto = "Código:\n```\nconst x = 1;\n```";
        assert_eq!(extraer_codigo(texto), "const x = 1;");
    }

    #[test]
    fn test_extraer_codigo_sin_bloque_devuelve_vacio() {
        let texto = "Solo texto sin código";
        // Cambio de comportamiento clave: ahora devuelve vacío para no romper archivos
        assert_eq!(extraer_codigo(texto), "");
    }

    #[test]
    fn test_extraer_codigo_opcional() {
        let texto = "```rust\nfn main() {}\n```";
        assert_eq!(extraer_codigo_opcional(texto), Some("fn main() {}".to_string()));
        
        let texto_limpio = "hola";
        assert_eq!(extraer_codigo_opcional(texto_limpio), None);
    }

    #[test]
    fn test_eliminar_bloques_codigo() {
        let texto = "Problema:\n```rust\nfn test() {}\n```\nSolución.";
        let resultado = eliminar_bloques_codigo(texto);
        assert!(resultado.contains("Problema:"));
        assert!(resultado.contains("Solución."));
        assert!(resultado.contains("[... Código guardado en .suggested ...]"));
        assert!(!resultado.contains("fn test()"));
    }

    #[test]
    fn test_eliminar_multiples_bloques() {
        let texto = "Antes\n```\ncodigo1\n```\nMedio\n```\ncodigo2\n```\nDespués";
        let resultado = eliminar_bloques_codigo(texto);
        assert_eq!(
            resultado.matches("[... Código guardado en .suggested ...]").count(),
            2
        );
    }
}
