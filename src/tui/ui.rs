use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, HighlightSpacing, List, ListItem, Paragraph};

use super::app::{App, FIELD_LABELS, Form, Mode, Panel};
use crate::db::{Row, Table};
use crate::money::{format_cents, format_signed};

const DATE_WIDTH: usize = 10;
const HIGHLIGHT: &str = "> ";
const STACK_BELOW_WIDTH: u16 = 100;

pub fn render(frame: &mut Frame, app: &mut App) {
    let [totals_area, panels_area, footer_area] =
        Layout::vertical([Constraint::Length(4), Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    render_totals(frame, totals_area, app);

    let direction = if panels_area.width >= STACK_BELOW_WIDTH { Direction::Horizontal } else { Direction::Vertical };
    let [left, right] = Layout::default()
        .direction(direction)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .areas(panels_area);
    let focus = app.focus;
    let [first, second] = &mut app.panels;
    render_panel(frame, left, first, focus == 0);
    render_panel(frame, right, second, focus == 1);

    let hint = match app.mode {
        Mode::Normal => "Tab/←→ painel · ↑↓ seleciona · s ordena · a adiciona · e edita · d apaga · q sai",
        Mode::Form(_) => "Tab muda de campo · Enter guarda · Esc cancela",
        Mode::ConfirmDelete => "s confirma · n cancela",
    };
    frame.render_widget(Paragraph::new(hint).style(Style::new().fg(Color::DarkGray)), footer_area);

    match &app.mode {
        Mode::Form(form) => render_form(frame, form),
        Mode::ConfirmDelete => render_confirm(frame, app),
        Mode::Normal => {}
    }
}

fn render_totals(frame: &mut Frame, area: Rect, app: &App) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let diff_color = if app.difference < 0 { Color::Red } else { Color::Green };
    let lines = vec![
        Line::from(vec![
            Span::styled(format!("Compras: {} ({})", format_cents(app.purchases.sum), app.purchases.count), bold),
            Span::raw("     "),
            Span::styled(format!("Vendas: {} ({})", format_cents(app.sales.sum), app.sales.count), bold),
        ]),
        Line::from(vec![
            Span::raw("Diferença (Vendas − Compras): "),
            Span::styled(format_signed(app.difference), bold.fg(diff_color)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).centered().block(Block::bordered().title(" flipbook ")), area);
}

fn title(table: Table) -> &'static str {
    match table {
        Table::Purchases => "Compras",
        Table::Sales => "Vendas",
    }
}

fn singular(table: Table) -> &'static str {
    match table {
        Table::Purchases => "compra",
        Table::Sales => "venda",
    }
}

fn render_panel(frame: &mut Frame, area: Rect, panel: &mut Panel, focused: bool) {
    let border = if focused { Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::new() };
    let block = Block::bordered()
        .title(format!(" {} — {} ", title(panel.table), panel.sort.label()))
        .border_style(border);

    // Sized to the widest price so the price stays next to the selection marker.
    let amount_width = panel.rows.iter().map(|row| format_cents(row.amount).chars().count()).max().unwrap_or(0);
    let description_width = (area.width as usize)
        .saturating_sub(2 + HIGHLIGHT.len() + amount_width + " - ".len() + 1 + DATE_WIDTH);
    let items: Vec<ListItem> = if panel.rows.is_empty() {
        vec![ListItem::new("Sem entradas")]
    } else {
        panel
            .rows
            .iter()
            .map(|row| ListItem::new(row_text(row, amount_width, description_width)))
            .collect()
    };

    // Selecting None would reset the scroll offset, so the unfocused panel keeps its selection unhighlighted.
    panel.list_state.select((!panel.rows.is_empty()).then_some(panel.selected));
    let (symbol, highlight) = if focused {
        (HIGHLIGHT, Style::new().add_modifier(Modifier::REVERSED))
    } else {
        ("  ", Style::new())
    };
    let list = List::new(items)
        .block(block)
        .highlight_symbol(symbol)
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(highlight);
    frame.render_stateful_widget(list, area, &mut panel.list_state);
}

fn row_text(row: &Row, amount_width: usize, description_width: usize) -> String {
    let description = truncate(&row.description, description_width);
    let date = row.date.as_deref().unwrap_or("");
    format!(
        "{:>amount_width$} - {description:<description_width$} {date:>DATE_WIDTH$}",
        format_cents(row.amount)
    )
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let kept: String = text.chars().take(width.saturating_sub(1)).collect();
    if width == 0 { kept } else { format!("{kept}…") }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
}

fn render_form(frame: &mut Frame, form: &Form) {
    let area = centered(frame.area(), 64, 9);
    let verb = if form.editing.is_some() { "Editar" } else { "Adicionar" };
    let block = Block::bordered()
        .title(format!(" {verb} {} ", singular(form.table)))
        .border_style(Style::new().fg(Color::Yellow));

    let mut lines: Vec<Line> = FIELD_LABELS
        .iter()
        .zip(&form.fields)
        .enumerate()
        .map(|(index, (label, value))| {
            let focused = index == form.focus;
            let style = if focused { Style::new().add_modifier(Modifier::BOLD) } else { Style::new() };
            let cursor = if focused { "▏" } else { "" };
            Line::styled(format!("{label:<10} {value}{cursor}"), style)
        })
        .collect();
    lines.push(Line::raw(""));
    if let Some(error) = &form.error {
        lines.push(Line::styled(error.clone(), Style::new().fg(Color::Red)));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_confirm(frame: &mut Frame, app: &App) {
    let panel = &app.panels[app.focus];
    let description = panel.rows.get(panel.selected).map_or("", |row| row.description.as_str());
    let area = centered(frame.area(), 50, 5);
    let block = Block::bordered().title(" Apagar ").border_style(Style::new().fg(Color::Red));
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![Line::raw(format!("Apagar «{description}»?")), Line::raw("(s/n)")]).block(block),
        area,
    );
}
