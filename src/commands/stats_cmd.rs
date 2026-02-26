use std::path::Path;
use std::fs;
use std::collections::BTreeMap;
use chrono::{Local, Datelike};
use serde::{Deserialize, Serialize};
use colored::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub sugerencias_aplicadas: u32,
    pub bugs_evitados: u32,
}

pub fn handle_stats_command(project_root: &Path, reset: Option<String>) {
    let stats_path = project_root.join(".sentinel_stats.json");
    let history_path = project_root.join(".sentinel/stats_history.json");

    if let Some(reset_period) = reset {
        reset_stats(&history_path, &reset_period);
        return;
    }

    // Mostrar estadísticas
    if !stats_path.exists() {
        println!("ℹ️  No hay estadísticas aún. Usa 'sentinel monitor' para empezar.");
        return;
    }

    let stats_content = fs::read_to_string(&stats_path).unwrap_or_default();
    let stats: crate::stats::SentinelStats =
        serde_json::from_str(&stats_content).unwrap_or_default();

    println!("\n{}", "📊 ESTADÍSTICAS DE SENTINEL".cyan().bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("\n{} TOTAL ACUMULADO", "📈".cyan());
    println!("  Tokens usados:         {:>10}", format!("{}", stats.total_tokens_used).green());
    println!("  Costo (Claude 4.5):    {:>10}", format!("${:.4}", stats.total_cost_usd).yellow());
    println!("  Sugerencias aplicadas: {:>10}", format!("{}", stats.sugerencias_aplicadas).bright_blue());
    println!("  Bugs evitados:         {:>10}", format!("{}", stats.bugs_criticos_evitados).green());

    // Cargar histórico
    if let Ok(history_content) = fs::read_to_string(&history_path) {
        if let Ok(history) = serde_json::from_str::<BTreeMap<String, HistoryEntry>>(&history_content) {
            let today = Local::now().format("%Y-%m-%d").to_string();
            let current_week = Local::now().format("%Y-W%V").to_string();
            let current_month = Local::now().format("%Y-%m").to_string();

            println!("\n{} HOY ({})", "📅".cyan(), today);
            if let Some(entry) = history.get(&today) {
                println!("  Tokens:  {}", entry.total_tokens);
                println!("  Costo:   ${:.4}", entry.total_cost_usd);
            } else {
                println!("  Sin estadísticas hoy");
            }

            println!("\n{} ESTA SEMANA ({})", "📆".cyan(), current_week);
            let week_stats = get_week_stats(&history, &current_week);
            println!("  Tokens:  {}", week_stats.0);
            println!("  Costo:   ${:.4}", week_stats.1);

            println!("\n{} ESTE MES ({})", "📅".cyan(), current_month);
            let month_stats = get_month_stats(&history, &current_month);
            println!("  Tokens:  {}", month_stats.0);
            println!("  Costo:   ${:.4}", month_stats.1);
        }
    }

    println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\n💡 Para resetear estadísticas:");
    println!("   sentinel stats --reset day      (Resetear hoy)");
    println!("   sentinel stats --reset week     (Resetear esta semana)");
    println!("   sentinel stats --reset month    (Resetear este mes)");
    println!("   sentinel stats --reset all      (Resetear TODO)");
    println!();
}

fn reset_stats(history_path: &Path, period: &str) {
    let history_dir = history_path.parent().unwrap();
    let _ = fs::create_dir_all(history_dir);

    let mut history: BTreeMap<String, HistoryEntry> = if let Ok(content) = fs::read_to_string(&history_path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    match period {
        "day" => {
            let today = Local::now().format("%Y-%m-%d").to_string();
            history.remove(&today);
            println!("✅ Estadísticas de hoy reseteadas");
        }
        "week" => {
            let current_week = Local::now().format("%Y-W%V").to_string();
            history.retain(|k, _| !k.starts_with(&current_week[..4]) || !k.contains(&current_week[5..]));
            println!("✅ Estadísticas de esta semana reseteadas");
        }
        "month" => {
            let current_month = Local::now().format("%Y-%m").to_string();
            history.retain(|k, _| !k.starts_with(&current_month));
            println!("✅ Estadísticas de este mes reseteadas");
        }
        "all" => {
            history.clear();
            println!("✅ Todas las estadísticas reseteadas");
        }
        _ => {
            println!("❌ Período no válido. Usa: day, week, month, all");
        }
    }

    if let Ok(content) = serde_json::to_string_pretty(&history) {
        let _ = fs::write(&history_path, content);
    }
}

fn get_week_stats(history: &BTreeMap<String, HistoryEntry>, week: &str) -> (u64, f64) {
    history
        .iter()
        .filter(|(date, _)| {
            let entry_week = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .ok()
                .map(|d| format!("{}-W{:02}", d.year(), d.iso_week().week()))
                .unwrap_or_default();
            entry_week == week
        })
        .fold((0u64, 0.0f64), |(tokens, cost), (_, entry)| {
            (tokens + entry.total_tokens, cost + entry.total_cost_usd)
        })
}

fn get_month_stats(history: &BTreeMap<String, HistoryEntry>, month: &str) -> (u64, f64) {
    history
        .iter()
        .filter(|(date, _)| date.starts_with(month))
        .fold((0u64, 0.0f64), |(tokens, cost), (_, entry)| {
            (tokens + entry.total_tokens, cost + entry.total_cost_usd)
        })
}
