mod db;
mod money;
mod snapshot;
mod tui;

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    /// Caminho da base de dados (por omissão, na pasta de dados do utilizador).
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Escreve um snapshot JSON da base de dados.
    Export {
        #[arg(default_value = "flipbook-snapshot.json")]
        file: PathBuf,
    },
    /// Substitui o conteúdo da base de dados por um snapshot JSON.
    Restore {
        file: PathBuf,
        /// Não pede confirmação quando a base de dados já tem dados.
        #[arg(long)]
        yes: bool,
    },
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("não foi possível ler {}", path.display()))
}

fn confirm(question: &str) -> Result<bool> {
    eprint!("{question} [s/N] ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "s" | "sim"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = match cli.db {
        Some(path) => path,
        None => db::default_path()?,
    };

    match cli.command {
        Some(Command::Export { file }) => {
            let conn = db::open(&db_path)?;
            let snapshot = snapshot::export(&conn)?;
            let json = serde_json::to_string_pretty(&snapshot)?;
            std::fs::write(&file, json).with_context(|| format!("não foi possível escrever {}", file.display()))?;
            println!(
                "Exportado para {}: {} compras, {} vendas",
                file.display(),
                snapshot.purchases.len(),
                snapshot.sales.len()
            );
        }
        Some(Command::Restore { file, yes }) => {
            let snapshot: snapshot::Snapshot = serde_json::from_str(&read(&file)?)
                .with_context(|| format!("snapshot inválido: {}", file.display()))?;
            let mut conn = db::open(&db_path)?;
            let replace = yes || !db::has_rows(&conn)? || confirm("A base de dados já tem dados. Substituir?")?;
            snapshot::restore(&mut conn, &snapshot, replace)?;
            println!(
                "Restaurado: {} compras, {} vendas",
                snapshot.purchases.len(),
                snapshot.sales.len()
            );
        }
        None => tui::run(db::open(&db_path)?)?,
    }
    Ok(())
}
