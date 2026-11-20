mod app;
mod ui;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use rusqlite::Connection;

use app::App;

pub fn run(conn: Connection) -> Result<()> {
    let mut app = App::new(conn)?;
    // `init` installs a panic hook that restores the terminal as well.
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::render(frame, app))?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key)?;
        }
    }
    Ok(())
}
