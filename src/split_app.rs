mod view;

use crate::{ui::util::stateful_list::StatefulList, Event, Term};

use splitmonic::wordlist::english::English;
use splitmonic::wordlist::Wordlist;

use crossbeam_channel::Receiver;
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use std::error::Error;

pub enum InputMode {
    Normal,
    Editing,
}

pub enum Screen {
    Input(InputMode),
    List,
}

pub struct SplitApp {
    rx: Receiver<Event<KeyEvent>>,

    pub autocomplete: &'static str,
    pub input: String,
    pub screen: Screen,
    pub mnemonic: StatefulList<String>,
    pub should_quit: bool,
}

impl SplitApp {
    pub fn new(rx: Receiver<Event<KeyEvent>>) -> Self {
        Self {
            rx,
            autocomplete: English::get_word(0).unwrap(),
            input: String::new(),
            screen: Screen::Input(InputMode::Normal),
            mnemonic: StatefulList::new(),
            should_quit: false,
        }
    }

    pub fn start_event_loop(&mut self, mut terminal: Term) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| view::draw(self, f))?;

            match self.rx.recv()? {
                Event::Input(event) => match self.screen {
                    Screen::Input(InputMode::Normal) => self.update_input_in_normal(event),
                    Screen::Input(InputMode::Editing) => self.update_input_in_editing(event),
                    Screen::List => self.update_in_list(event),
                },
                Event::Tick => {}
            }

            if self.should_quit {
                terminal::disable_raw_mode()?;
                execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen,)?;
                terminal.show_cursor()?;
                break;
            }
        }

        Ok(())
    }

    fn update_input_in_editing(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char(char) => {
                self.input.push(char);

                match English::starting_with(&self.input).as_slice() {
                    [] => {
                        self.autocomplete = "";
                        self.input.pop();
                    }
                    [only_one] => {
                        self.autocomplete = "";
                        self.mnemonic.push(only_one.to_string());
                        self.input = "".to_string();
                    }
                    [head, ..] => self.autocomplete = head,
                }
            }
            KeyCode::Esc => self.screen = Screen::Input(InputMode::Normal),
            KeyCode::Backspace => {
                self.input.pop();

                match English::starting_with(&self.input).as_slice() {
                    [] => self.autocomplete = "",
                    [head, ..] => self.autocomplete = head,
                }
            }
            KeyCode::Right => self.input = self.autocomplete.to_string(),
            KeyCode::Down => {
                self.mnemonic.select();
                self.screen = Screen::List;
            }
            KeyCode::Tab => {
                if let Some(word) = English::next_starting_with(&self.input, &self.autocomplete) {
                    self.autocomplete = word;
                }
            }
            KeyCode::Enter => {
                self.input = self.input.trim().to_string();
                self.mnemonic.push(self.autocomplete.to_string());
                self.input = "".to_string();
            }
            _ => {}
        }
    }

    fn update_input_in_normal(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('i') => self.screen = Screen::Input(InputMode::Editing),
            KeyCode::Esc => self.screen = Screen::Input(InputMode::Normal),
            KeyCode::Down | KeyCode::Tab => {
                self.mnemonic.select();
                self.screen = Screen::List;
            }
            KeyCode::Up => {
                self.mnemonic.previous();
            }
            _ => {}
        }
    }

    fn update_in_list(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('i') => {
                self.mnemonic.unselect();
                self.screen = Screen::Input(InputMode::Editing)
            }
            KeyCode::Esc | KeyCode::Tab => {
                self.mnemonic.unselect();
                self.screen = Screen::Input(InputMode::Normal)
            }
            KeyCode::Up if key_event.modifiers.contains(KeyModifiers::ALT) => {
                self.mnemonic.move_up();
            }

            KeyCode::Up => {
                if self.mnemonic.items.is_empty() {
                    self.mnemonic.unselect();
                    self.screen = Screen::Input(InputMode::Normal)
                } else {
                    self.mnemonic.previous()
                }
            }
            KeyCode::Char('d') => self.mnemonic.delete_selected(),
            KeyCode::Down if key_event.modifiers.contains(KeyModifiers::ALT) => {
                self.mnemonic.move_down();
            }
            KeyCode::Down => self.mnemonic.next(),
            _ => {}
        }
    }
}
