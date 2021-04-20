use crossbeam_channel::unbounded;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute, terminal,
};
use std::{
    error::Error,
    io, thread,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
    Frame, Terminal,
};

use unicode_width::UnicodeWidthStr;

enum Event<I> {
    Input(I),
    Tick,
}

enum InputMode {
    Normal,
    Editing,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            should_quit: false,
        }
    }
}

struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
    /// Set when app is to quit,
    should_quit: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();

    // Setup input handling
    let (tx, rx) = unbounded();

    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }

            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;

    loop {
        terminal.draw(|f| draw(f, &mut app))?;
        match rx.recv()? {
            Event::Input(event) => match &app.input_mode {
                InputMode::Normal => handle_input_in_normal(event.code, &mut app),
                InputMode::Editing => handle_input_in_editing(event.code, &mut app),
            },
            Event::Tick => {}
        }

        if app.should_quit {
            terminal::disable_raw_mode()?;
            execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen,)?;
            terminal.show_cursor()?;
            break;
        }
    }

    Ok(())
}

fn handle_input_in_editing(keycode: KeyCode, app: &mut App) {
    match keycode {
        KeyCode::Char(char) => app.input.push(char),
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Enter => {
            app.messages.push(app.input.clone());
            app.input = "".to_string();
        }
        _ => {}
    }
}

fn handle_input_in_normal(keycode: KeyCode, app: &mut App) {
    match keycode {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('i') => app.input_mode = InputMode::Editing,
        _ => {}
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("i", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    };

    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);

    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[1]);

    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
    }

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
            ListItem::new(content)
        })
        .collect();

    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));

    f.render_widget(messages, chunks[2]);
}
