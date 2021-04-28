mod split_app;
mod ui;

use crate::split_app::SplitApp;
use crossbeam_channel::unbounded;
use crossterm::{
    event::{self, Event as CEvent},
    execute, terminal,
};
use eyre::Result;
use std::{
    io::{self, Stdout},
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};

pub type Backend = CrosstermBackend<Stdout>;
pub type Term = Terminal<Backend>;

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup input handling
    let (tx, rx) = unbounded();
    let mut split_app = SplitApp::new(tx.clone(), rx);

    let tick_rate = Duration::from_secs(5);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(split_app::Event::Input(key)).unwrap();
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx.send(split_app::Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;
    split_app.start_event_loop(terminal)?;

    Ok(())
}
