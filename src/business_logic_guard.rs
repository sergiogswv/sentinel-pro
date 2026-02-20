use std::path::Path;
use std::process::Command;

/// Obtiene el contenido del archivo en HEAD (último commit).
/// Retorna None si el archivo no está en git o si git no está disponible.
pub fn get_git_previous_content(file_path: &Path, project_root: &Path) -> Option<String> {
    let rel_path = file_path.strip_prefix(project_root).ok()?;
    let rel_str = rel_path.to_string_lossy();

    let output = Command::new("git")
        .args(["show", &format!("HEAD:{}", rel_str)])
        .current_dir(project_root)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

/// Compara dos versiones del código y retorna un diff legible para el AI.
/// Retorna None si los archivos son idénticos o si prev_content es None.
pub fn build_regression_context(prev_content: &str, new_content: &str) -> Option<String> {
    if prev_content == new_content {
        return None;
    }

    let prev_lines: Vec<&str> = prev_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let removed: Vec<&str> = prev_lines.iter()
        .filter(|l| !new_lines.contains(l))
        .copied()
        .collect();

    let added: Vec<&str> = new_lines.iter()
        .filter(|l| !prev_lines.contains(l))
        .copied()
        .collect();

    if removed.is_empty() && added.is_empty() {
        return None;
    }

    let mut diff = String::from("CAMBIOS DETECTADOS (diff simplificado):\n");
    if !removed.is_empty() {
        diff.push_str("\nLÍNEAS ELIMINADAS:\n");
        for line in removed.iter().take(30) {
            diff.push_str(&format!("- {}\n", line));
        }
    }
    if !added.is_empty() {
        diff.push_str("\nLÍNEAS AGREGADAS:\n");
        for line in added.iter().take(30) {
            diff.push_str(&format!("+ {}\n", line));
        }
    }

    Some(diff)
}

/// Construye el prompt para detectar regresiones de lógica de negocio.
pub fn build_regression_prompt(diff_context: &str, file_name: &str) -> String {
    format!(
        "Actúa como un revisor de código experto en detección de regresiones.\n\
        Analiza los siguientes cambios en el archivo '{}' y determina:\n\
        1. ¿Se eliminó alguna validación, guard clause o lógica de negocio importante?\n\
        2. ¿Se cambió el comportamiento de alguna función de forma que pueda romper contratos existentes?\n\
        3. ¿Se eliminó manejo de errores o casos edge?\n\
        \n\
        Responde con:\n\
        - REGRESION_DETECTADA: [descripción concisa] si hay una regresión clara\n\
        - SIN_REGRESION si los cambios son seguros\n\
        - REVISAR: [motivo] si hay algo sospechoso pero no definitivo\n\
        \n\
        CAMBIOS A ANALIZAR:\n\
        {}\n",
        file_name, diff_context
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_regression_context_identical_files_returns_none() {
        let content = "function foo() { return 1; }";
        assert!(build_regression_context(content, content).is_none());
    }

    #[test]
    fn test_build_regression_context_detects_removed_lines() {
        let prev = "function validate(email) {\n  if (!email) throw new Error('required');\n  return true;\n}";
        let new = "function validate(email) {\n  return true;\n}";
        let result = build_regression_context(prev, new);
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert!(ctx.contains("ELIMINADAS"));
        assert!(ctx.contains("throw new Error"));
    }

    #[test]
    fn test_build_regression_context_detects_added_lines() {
        let prev = "function foo() { return 1; }";
        let new = "function foo() { return 1; }\nfunction bar() { return 2; }";
        let result = build_regression_context(prev, new);
        assert!(result.is_some());
        assert!(result.unwrap().contains("AGREGADAS"));
    }

    #[test]
    fn test_build_regression_prompt_contains_file_name() {
        let prompt = build_regression_prompt("- old line\n+ new line", "user.service.ts");
        assert!(prompt.contains("user.service.ts"));
        assert!(prompt.contains("REGRESION_DETECTADA"));
        assert!(prompt.contains("SIN_REGRESION"));
    }
}
