use crate::config::SentinelConfig;
use crate::index::{IndexDb, ProjectIndexBuilder};
use colored::Colorize;
use std::sync::Arc;

pub fn handle_index_command(rebuild: bool, check: bool) {
    let project_root = std::env::current_dir().unwrap();
    let config = SentinelConfig::load(&project_root).unwrap_or_default();
    let index_path = project_root.join(".sentinel/index.db");
    let index_db = IndexDb::open(&index_path).ok().map(Arc::new);

    let Some(db) = index_db else {
        println!(
            "{} No se encontró el directorio .sentinel. Corre `sentinel pro check` primero.",
            "❌".red()
        );
        return;
    };

    if !rebuild && !check {
        println!("Uso: sentinel index --check | --rebuild");
        return;
    }

    if check {
        print_index_status(&db, &project_root, &config.file_extensions);
    }

    if rebuild {
        println!("\n{}", "🔄 Reconstruyendo índice desde cero...".bold());
        db.clear_all().expect("Error limpiando el índice");
        let builder = ProjectIndexBuilder::new(Arc::clone(&db));
        builder
            .index_project(&project_root, &config.file_extensions)
            .expect("Error indexando el proyecto");
        let count = db.indexed_file_count();
        println!(
            "{} Índice reconstruido. {} archivos indexados.",
            "✅".green(),
            count.to_string().cyan()
        );
    }
}

fn print_index_status(db: &IndexDb, project_root: &std::path::Path, extensions: &[String]) {
    let disk_count = count_project_files(project_root, extensions);
    let index_count = db.indexed_file_count();
    let diff = (disk_count as isize - index_count as isize).unsigned_abs();
    let stale_threshold = 5.max(disk_count / 10);
    let stale = diff > stale_threshold;

    let conn = db.lock();
    let last_indexed: Option<String> = conn
        .query_row("SELECT MAX(last_indexed) FROM file_index", [], |row| row.get(0))
        .ok()
        .flatten();
    drop(conn);

    println!("\n{}", "📊 Estado del índice:".bold());
    println!("   Archivos indexados:  {}", index_count.to_string().cyan());
    if disk_count > index_count {
        println!(
            "   Archivos en disco:   {}",
            format!("{}  (+{} no indexados)", disk_count, diff)
                .yellow()
                .to_string()
        );
    } else {
        println!("   Archivos en disco:   {}", disk_count.to_string().green());
    }
    println!(
        "   Último indexado:     {}",
        last_indexed.unwrap_or_else(|| "nunca".to_string())
    );
    println!(
        "   Estado:              {}",
        if stale {
            "⚠️  Desactualizado".yellow().to_string()
        } else {
            "✅ Al día".green().to_string()
        }
    );
    if stale {
        println!(
            "\n   Corre {} para actualizar.",
            "`sentinel index --rebuild`".cyan()
        );
    }
}

pub fn count_project_files(root: &std::path::Path, extensions: &[String]) -> usize {
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| extensions.contains(&x.to_string()))
                    .unwrap_or(false)
        })
        .count()
}
