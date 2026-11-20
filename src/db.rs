use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Local, NaiveDate};
use rusqlite::Connection;

use crate::money::parse_cents;

#[derive(Clone, Copy)]
pub enum Table {
    Purchases,
    Sales,
}

impl Table {
    // Only ever interpolated into SQL, so it must stay a fixed identifier.
    pub fn name(self) -> &'static str {
        match self {
            Table::Purchases => "purchases",
            Table::Sales => "sales",
        }
    }
}

#[derive(Clone, Copy)]
pub enum Sort {
    Newest,
    MostExpensive,
    Cheapest,
}

impl Sort {
    pub fn next(self) -> Sort {
        match self {
            Sort::Newest => Sort::MostExpensive,
            Sort::MostExpensive => Sort::Cheapest,
            Sort::Cheapest => Sort::Newest,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Newest => "Mais recentes",
            Sort::MostExpensive => "Mais caras",
            Sort::Cheapest => "Mais baratas",
        }
    }

    fn order_by(self) -> &'static str {
        match self {
            Sort::Newest => "date IS NULL, date DESC, id DESC",
            Sort::MostExpensive => "amount DESC, id DESC",
            Sort::Cheapest => "amount ASC, id DESC",
        }
    }
}

/// Raw form fields; `insert` and `update` validate them before anything is written.
pub struct EntryInput {
    pub description: String,
    pub amount: String,
    pub date: String,
    pub notes: String,
}

#[derive(Debug, PartialEq)]
pub struct Row {
    pub id: i64,
    pub description: String,
    pub amount: i64,
    pub notes: Option<String>,
    pub date: Option<String>,
}

struct Valid {
    description: String,
    amount: i64,
    notes: Option<String>,
    date: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Totals {
    pub sum: i64,
    pub count: i64,
}

pub fn default_path() -> Result<PathBuf> {
    let dir = dirs::data_dir().context("não foi possível encontrar a pasta de dados do utilizador")?;
    Ok(dir.join("flipbook").join("flipbook.db"))
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).with_context(|| format!("não foi possível criar {}", parent.display()))?;
    }
    init(Connection::open(path)?)
}

fn init(conn: Connection) -> Result<Connection> {
    conn.pragma_update(None, "foreign_keys", true)?;
    for table in [Table::Purchases, Table::Sales] {
        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    description TEXT NOT NULL,
                    amount      INTEGER NOT NULL CHECK (amount >= 0),
                    notes       TEXT,
                    date        TEXT
                )",
                table.name()
            ),
            [],
        )?;
    }
    Ok(conn)
}

pub fn totals(conn: &Connection, table: Table) -> Result<Totals> {
    let sql = format!("SELECT COALESCE(SUM(amount), 0), COUNT(*) FROM {}", table.name());
    Ok(conn.query_row(&sql, [], |row| Ok(Totals { sum: row.get(0)?, count: row.get(1)? }))?)
}

pub fn has_rows(conn: &Connection) -> Result<bool> {
    Ok(totals(conn, Table::Purchases)?.count + totals(conn, Table::Sales)?.count > 0)
}

/// Sales minus purchases, in cents.
pub fn difference(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT (SELECT COALESCE(SUM(amount), 0) FROM sales) - (SELECT COALESCE(SUM(amount), 0) FROM purchases)",
        [],
        |row| row.get(0),
    )?)
}

fn validate(input: &EntryInput) -> Result<Valid> {
    let description = input.description.trim();
    if description.is_empty() {
        bail!("a descrição é obrigatória");
    }
    let amount = parse_cents(&input.amount)?;
    let date = match input.date.trim() {
        "" => None,
        text => Some(
            NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .map_err(|_| anyhow!("data inválida (use aaaa-mm-dd): {text:?}"))?
                .to_string(),
        ),
    };
    let notes = Some(input.notes.trim()).filter(|n| !n.is_empty()).map(String::from);
    Ok(Valid { description: description.to_string(), amount, notes, date })
}

/// Adds an entry; an empty date becomes today.
pub fn insert(conn: &Connection, table: Table, input: &EntryInput) -> Result<i64> {
    let valid = validate(input)?;
    let date = valid.date.unwrap_or_else(|| Local::now().date_naive().to_string());
    conn.execute(
        &format!(
            "INSERT INTO {} (description, amount, notes, date) VALUES (?1, ?2, ?3, ?4)",
            table.name()
        ),
        (&valid.description, valid.amount, &valid.notes, &date),
    )?;
    Ok(conn.last_insert_rowid())
}

/// Edits an entry; an empty date is kept empty (imported entries have none).
pub fn update(conn: &Connection, table: Table, id: i64, input: &EntryInput) -> Result<()> {
    let valid = validate(input)?;
    conn.execute(
        &format!(
            "UPDATE {} SET description = ?1, amount = ?2, notes = ?3, date = ?4 WHERE id = ?5",
            table.name()
        ),
        (&valid.description, valid.amount, &valid.notes, &valid.date, id),
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, table: Table, id: i64) -> Result<()> {
    conn.execute(&format!("DELETE FROM {} WHERE id = ?1", table.name()), [id])?;
    Ok(())
}

pub fn list(conn: &Connection, table: Table, sort: Sort) -> Result<Vec<Row>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT id, description, amount, notes, date FROM {} ORDER BY {}",
        table.name(),
        sort.order_by()
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(Row {
            id: row.get(0)?,
            description: row.get(1)?,
            amount: row.get(2)?,
            notes: row.get(3)?,
            date: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
