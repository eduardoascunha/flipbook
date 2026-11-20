use anyhow::Result;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;
use rusqlite::Connection;

use crate::db::{self, EntryInput, Row, Sort, Table, Totals};
use crate::money::format_decimal;

pub const FIELD_LABELS: [&str; 4] = ["Descrição", "Valor", "Data", "Notas"];

const PAGE: usize = 10;

pub struct Panel {
    pub table: Table,
    pub sort: Sort,
    pub rows: Vec<Row>,
    pub selected: usize,
    pub list_state: ListState,
}

pub struct Form {
    pub table: Table,
    pub editing: Option<i64>,
    pub fields: [String; 4],
    pub focus: usize,
    pub error: Option<String>,
}

pub enum Mode {
    Normal,
    Form(Form),
    ConfirmDelete,
}

pub struct App {
    conn: Connection,
    pub panels: [Panel; 2],
    pub focus: usize,
    pub purchases: Totals,
    pub sales: Totals,
    pub difference: i64,
    pub mode: Mode,
    pub quit: bool,
}

impl App {
    pub fn new(conn: Connection) -> Result<App> {
        let panel = |table| Panel {
            table,
            sort: Sort::Newest,
            rows: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
        };
        let mut app = App {
            conn,
            panels: [panel(Table::Purchases), panel(Table::Sales)],
            focus: 0,
            purchases: Totals { sum: 0, count: 0 },
            sales: Totals { sum: 0, count: 0 },
            difference: 0,
            mode: Mode::Normal,
            quit: false,
        };
        app.refresh()?;
        Ok(app)
    }

    fn refresh(&mut self) -> Result<()> {
        for panel in &mut self.panels {
            panel.rows = db::list(&self.conn, panel.table, panel.sort)?;
            panel.selected = panel.selected.min(panel.rows.len().saturating_sub(1));
        }
        self.purchases = db::totals(&self.conn, Table::Purchases)?;
        self.sales = db::totals(&self.conn, Table::Sales)?;
        self.difference = db::difference(&self.conn)?;
        Ok(())
    }

    fn selected_row(&self) -> Option<&Row> {
        let panel = &self.panels[self.focus];
        panel.rows.get(panel.selected)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Normal => self.normal_key(key.code),
            Mode::Form(form) => self.form_key(form, key.code),
            Mode::ConfirmDelete => self.confirm_key(key.code),
        }
    }

    fn normal_key(&mut self, code: KeyCode) -> Result<()> {
        let focus = self.focus;
        match code {
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => self.focus = 1 - focus,
            KeyCode::Up => {
                let panel = &mut self.panels[focus];
                panel.selected = panel.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let panel = &mut self.panels[focus];
                if panel.selected + 1 < panel.rows.len() {
                    panel.selected += 1;
                }
            }
            KeyCode::PageUp => {
                let panel = &mut self.panels[focus];
                panel.selected = panel.selected.saturating_sub(PAGE);
            }
            KeyCode::PageDown => {
                let panel = &mut self.panels[focus];
                panel.selected = (panel.selected + PAGE).min(panel.rows.len().saturating_sub(1));
            }
            KeyCode::Char('s') => {
                let panel = &mut self.panels[focus];
                panel.sort = panel.sort.next();
                panel.selected = 0;
                self.refresh()?;
            }
            KeyCode::Char('a') => {
                let today = Local::now().date_naive().to_string();
                self.mode = Mode::Form(Form {
                    table: self.panels[focus].table,
                    editing: None,
                    fields: [String::new(), String::new(), today, String::new()],
                    focus: 0,
                    error: None,
                });
            }
            KeyCode::Char('e') => {
                if let Some(row) = self.selected_row() {
                    self.mode = Mode::Form(Form {
                        table: self.panels[focus].table,
                        editing: Some(row.id),
                        fields: [
                            row.description.clone(),
                            format_decimal(row.amount),
                            row.date.clone().unwrap_or_default(),
                            row.notes.clone().unwrap_or_default(),
                        ],
                        focus: 0,
                        error: None,
                    });
                }
            }
            KeyCode::Char('d') => {
                if self.selected_row().is_some() {
                    self.mode = Mode::ConfirmDelete;
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            _ => {}
        }
        Ok(())
    }

    fn form_key(&mut self, mut form: Form, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Esc => return Ok(()),
            KeyCode::Tab => form.focus = (form.focus + 1) % form.fields.len(),
            KeyCode::Backspace => {
                form.fields[form.focus].pop();
            }
            KeyCode::Char(c) => form.fields[form.focus].push(c),
            KeyCode::Enter => {
                let [description, amount, date, notes] = form.fields.clone();
                let input = EntryInput { description, amount, date, notes };
                let saved = match form.editing {
                    Some(id) => db::update(&self.conn, form.table, id, &input),
                    None => db::insert(&self.conn, form.table, &input).map(|_| ()),
                };
                match saved {
                    Ok(()) => return self.refresh(),
                    Err(err) => form.error = Some(err.to_string()),
                }
            }
            _ => {}
        }
        self.mode = Mode::Form(form);
        Ok(())
    }

    fn confirm_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('s') => {
                if let Some(row) = self.selected_row() {
                    db::delete(&self.conn, self.panels[self.focus].table, row.id)?;
                    self.refresh()?;
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {}
            _ => self.mode = Mode::ConfirmDelete,
        }
        Ok(())
    }
}
