use crossbeam_channel::unbounded;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent, KeyModifiers},
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
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};

use splitmonic::wordlist::english::English;
use splitmonic::wordlist::Wordlist;

use unicode_width::UnicodeWidthStr;

enum Event<I> {
    Input(I),
    Tick,
}

enum InputMode {
    Normal,
    Editing,
}

enum ScreenState {
    Input(InputMode),
    List,
}

impl Default for App {
    fn default() -> App {
        App {
            autocomplete: English::get_word(0).unwrap(),
            input: String::new(),
            screen_state: ScreenState::Input(InputMode::Normal),
            messages: StatefulList::new(),
            should_quit: false,
        }
    }
}

struct App {
    autocomplete: &'static str,
    input: String,
    screen_state: ScreenState,
    messages: StatefulList<String>,
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
            Event::Input(event) => match &app.screen_state {
                ScreenState::Input(InputMode::Normal) => handle_input_in_normal(event, &mut app),

                ScreenState::Input(InputMode::Editing) => handle_input_in_editing(event, &mut app),

                ScreenState::List => handle_list(event, &mut app),
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

fn handle_input_in_editing(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Char(char) => {
            app.input.push(char);

            match English::starting_with(&app.input).as_slice() {
                [] => {
                    app.autocomplete = "";
                    app.input.pop();
                }
                [only_one] => {
                    app.autocomplete = "";
                    app.messages.push(only_one.to_string());
                    app.input = "".to_string();
                }
                [head, ..] => app.autocomplete = head,
            }
        }
        KeyCode::Esc => app.screen_state = ScreenState::Input(InputMode::Normal),
        KeyCode::Backspace => {
            app.input.pop();

            match English::starting_with(&app.input).as_slice() {
                [] => app.autocomplete = "",
                [head, ..] => app.autocomplete = head,
            }
        }
        KeyCode::Right => app.input = app.autocomplete.to_string(),
        KeyCode::Down => {
            app.messages.select();
            app.screen_state = ScreenState::List;
        }
        KeyCode::Tab => {
            if let Some(word) = English::next_starting_with(&app.input, &app.autocomplete) {
                app.autocomplete = word;
            }
        }
        KeyCode::Enter => {
            app.input = app.input.trim().to_string();
            app.messages.push(app.autocomplete.to_string());
            app.input = "".to_string();
        }
        _ => {}
    }
}

fn handle_input_in_normal(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('i') => app.screen_state = ScreenState::Input(InputMode::Editing),
        KeyCode::Esc => app.screen_state = ScreenState::Input(InputMode::Normal),
        KeyCode::Down | KeyCode::Tab => {
            app.messages.select();
            app.screen_state = ScreenState::List;
        }
        KeyCode::Up => {
            app.messages.previous();
        }
        _ => {}
    }
}

fn handle_list(key_event: KeyEvent, app: &mut App) {
    match key_event.code {
        KeyCode::Char('i') => {
            app.messages.unselect();
            app.screen_state = ScreenState::Input(InputMode::Editing)
        }
        KeyCode::Esc | KeyCode::Tab => {
            app.messages.unselect();
            app.screen_state = ScreenState::Input(InputMode::Normal)
        }
        KeyCode::Up if key_event.modifiers.contains(KeyModifiers::ALT) => {
            app.messages.move_up();
        }

        KeyCode::Up => {
            if app.messages.items.is_empty() {
                app.messages.unselect();
                app.screen_state = ScreenState::Input(InputMode::Normal)
            } else {
                app.messages.previous()
            }
        }
        KeyCode::Char('d') => app.messages.delete_selected(),
        KeyCode::Down if key_event.modifiers.contains(KeyModifiers::ALT) => {
            app.messages.move_down();
        }
        KeyCode::Down => app.messages.next(),
        _ => {}
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let help_box_size = match &app.screen_state {
        ScreenState::List => 3,
        _ => 1,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Min(help_box_size + 1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (mut text, style) = match app.screen_state {
        ScreenState::Input(InputMode::Normal) => (
            Text::from(Spans::from(vec![
                Span::raw("Press "),
                Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to exit, "),
                Span::styled("i ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to start editing, "),
                Span::styled("↓ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("or "),
                Span::styled("<TAB> ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to access the word list"),
            ])),
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),

        ScreenState::Input(InputMode::Editing) => (
            Text::from(Spans::from(vec![
                Span::raw("Press "),
                Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to add the word, "),
                Span::styled("↓ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to access the word list, "),
                Span::styled("<TAB> ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("to see the next autocomplete word"),
            ])),
            Style::default(),
        ),

        ScreenState::List => (
            {
                let mut texts = Text::from(Spans::from(vec![
                    Span::raw("Press "),
                    Span::styled("<TAB> ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to go to normal mode, "),
                    Span::styled("i ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to add new words, "),
                ]));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled(
                        "      <ALT> + ↓ ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("to move word down, "),
                    Span::styled("<ALT> + ↑ ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to move word up, "),
                ])));

                texts.extend(Text::from(Spans::from(vec![
                    Span::styled("      d ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to delete word, "),
                    Span::styled("e ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("to edit word "),
                ])));

                texts
            },
            Style::default(),
        ),
    };

    text.patch_style(style);

    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

    let input_text = match app.screen_state {
        ScreenState::Input(InputMode::Editing) => {
            let autocomplete = if app.autocomplete.len() >= app.input.len() {
                &app.autocomplete[app.input.len()..]
            } else {
                &app.autocomplete
            };

            vec![Spans::from(vec![
                Span::raw(&app.input),
                Span::styled(autocomplete, Style::default().fg(Color::DarkGray)),
            ])]
        }
        _ => vec![Spans::from(Span::raw(""))],
    };

    let input = Paragraph::new(input_text)
        .style(match app.screen_state {
            ScreenState::Input(InputMode::Editing) => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));

    f.render_widget(input, chunks[1]);

    match app.screen_state {
        ScreenState::List => {}

        ScreenState::Input(InputMode::Normal) => {}

        ScreenState::Input(InputMode::Editing) => {
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
        .items
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i + 1, m)))];
            ListItem::new(content)
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let messages = List::new(messages)
        .style(Style::default())
        .block(match app.screen_state {
            ScreenState::List => Block::default()
                .borders(Borders::ALL)
                .title("List")
                .border_style(Style::default().fg(Color::Yellow)),

            _ => Block::default()
                .borders(Borders::ALL)
                .title("List")
                .border_style(Style::default()),
        })
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        )
        .highlight_symbol("> ");

    // We can now render the item list
    f.render_stateful_widget(messages, chunks[2], &mut app.messages.state);
}

use tui::widgets::ListState;
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item)
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.is_empty() {
                    0
                } else if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.is_empty() {
                    0
                } else if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn move_up(&mut self) {
        match self.state.selected() {
            // top of list, move to the bottom
            Some(0) if self.items.len() >= 2 => {
                let new_index = self.items.len() - 1;

                let item = self.items.remove(0);
                self.items.insert(new_index, item);
                self.state.select(Some(new_index));
            }
            Some(index) if self.items.len() >= 2 => {
                let item = self.items.remove(index);
                self.items.insert(index - 1, item);
                self.state.select(Some(index - 1));
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.state.selected() {
            // bottom of the list, move to top
            Some(index) if self.items.len() - 1 == index => {
                let new_index = 0;

                let item = self.items.remove(index);
                self.items.insert(new_index, item);
                self.state.select(Some(new_index));
            }

            Some(index) if self.items.len() >= 2 => {
                let item = self.items.remove(index);
                self.items.insert(index + 1, item);
                self.state.select(Some(index + 1));
            }
            _ => {}
        }
    }

    pub fn select(&mut self) {
        if self.items.len() >= 1 {
            self.state.select(Some(0));
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn delete_selected(&mut self) {
        match self.state.selected() {
            Some(index) if self.items.len() > index => {
                self.items.remove(index);
            }
            _ => {}
        }
    }
}
