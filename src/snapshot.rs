use anyhow::{Result, bail};
use chrono::Local;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::{Table, has_rows};

const VERSION: u32 = 1;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub description: String,
    pub amount: i64,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub exported_at: String,
    pub purchases: Vec<SnapshotRow>,
    pub sales: Vec<SnapshotRow>,
}

fn read_rows(conn: &Connection, table: Table) -> Result<Vec<SnapshotRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT description, amount, notes, date FROM {} ORDER BY id",
        table.name()
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(SnapshotRow {
            description: row.get(0)?,
            amount: row.get(1)?,
            notes: row.get(2)?,
            date: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn write_rows(conn: &Connection, table: Table, rows: &[SnapshotRow]) -> Result<()> {
    let mut stmt = conn.prepare(&format!(
        "INSERT INTO {} (description, amount, notes, date) VALUES (?1, ?2, ?3, ?4)",
        table.name()
    ))?;
    for row in rows {
        stmt.execute((&row.description, row.amount, &row.notes, &row.date))?;
    }
    Ok(())
}

pub fn export(conn: &Connection) -> Result<Snapshot> {
    Ok(Snapshot {
        version: VERSION,
        exported_at: Local::now().to_rfc3339(),
        purchases: read_rows(conn, Table::Purchases)?,
        sales: read_rows(conn, Table::Sales)?,
    })
}

/// Replaces both tables with the snapshot; a non-empty database is only replaced when `replace` is true.
pub fn restore(conn: &mut Connection, snapshot: &Snapshot, replace: bool) -> Result<()> {
    if snapshot.version != VERSION {
        bail!("versão do snapshot desconhecida: {}", snapshot.version);
    }
    let tx = conn.transaction()?;
    if has_rows(&tx)? {
        if !replace {
            bail!("a base de dados já tem dados; nada foi substituído");
        }
        tx.execute("DELETE FROM purchases", [])?;
        tx.execute("DELETE FROM sales", [])?;
    }
    write_rows(&tx, Table::Purchases, &snapshot.purchases)?;
    write_rows(&tx, Table::Sales, &snapshot.sales)?;
    tx.commit()?;
    Ok(())
}
