mod ui;

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
use tui::{backend::CrosstermBackend, Terminal};

use splitmonic::wordlist::english::English;
use splitmonic::wordlist::Wordlist;

pub enum Event<I> {
    Input(I),
    Tick,
}

pub enum InputMode {
    Normal,
    Editing,
}

pub enum ScreenState {
    Input(InputMode),
    List,
}

impl Default for SplitApp {
    fn default() -> SplitApp {
        SplitApp {
            autocomplete: English::get_word(0).unwrap(),
            input: String::new(),
            screen_state: ScreenState::Input(InputMode::Normal),
            mnemonic: StatefulList::new(),
            should_quit: false,
        }
    }
}

pub struct SplitApp {
    autocomplete: &'static str,
    input: String,
    screen_state: ScreenState,
    mnemonic: StatefulList<String>,
    should_quit: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = SplitApp::default();

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
        terminal.draw(|f| ui::draw(f, &mut app))?;
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

fn handle_input_in_editing(key_event: KeyEvent, app: &mut SplitApp) {
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
                    app.mnemonic.push(only_one.to_string());
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
            app.mnemonic.select();
            app.screen_state = ScreenState::List;
        }
        KeyCode::Tab => {
            if let Some(word) = English::next_starting_with(&app.input, &app.autocomplete) {
                app.autocomplete = word;
            }
        }
        KeyCode::Enter => {
            app.input = app.input.trim().to_string();
            app.mnemonic.push(app.autocomplete.to_string());
            app.input = "".to_string();
        }
        _ => {}
    }
}

fn handle_input_in_normal(key_event: KeyEvent, app: &mut SplitApp) {
    match key_event.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('i') => app.screen_state = ScreenState::Input(InputMode::Editing),
        KeyCode::Esc => app.screen_state = ScreenState::Input(InputMode::Normal),
        KeyCode::Down | KeyCode::Tab => {
            app.mnemonic.select();
            app.screen_state = ScreenState::List;
        }
        KeyCode::Up => {
            app.mnemonic.previous();
        }
        _ => {}
    }
}

fn handle_list(key_event: KeyEvent, app: &mut SplitApp) {
    match key_event.code {
        KeyCode::Char('i') => {
            app.mnemonic.unselect();
            app.screen_state = ScreenState::Input(InputMode::Editing)
        }
        KeyCode::Esc | KeyCode::Tab => {
            app.mnemonic.unselect();
            app.screen_state = ScreenState::Input(InputMode::Normal)
        }
        KeyCode::Up if key_event.modifiers.contains(KeyModifiers::ALT) => {
            app.mnemonic.move_up();
        }

        KeyCode::Up => {
            if app.mnemonic.items.is_empty() {
                app.mnemonic.unselect();
                app.screen_state = ScreenState::Input(InputMode::Normal)
            } else {
                app.mnemonic.previous()
            }
        }
        KeyCode::Char('d') => app.mnemonic.delete_selected(),
        KeyCode::Down if key_event.modifiers.contains(KeyModifiers::ALT) => {
            app.mnemonic.move_down();
        }
        KeyCode::Down => app.mnemonic.next(),
        _ => {}
    }
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
                if self.items.is_empty() || i >= self.items.len() - 1 {
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
        if !self.items.is_empty() {
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
