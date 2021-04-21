mod view;

use crate::{ui::util::stateful_list::StatefulList, Effect, Event, Term};

use crossbeam_channel::{Receiver, Sender};
use splitmonic::wordlist::english::English;
use splitmonic::wordlist::Wordlist;

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use std::{borrow::Cow, error::Error};

pub enum InputMode {
    Normal,
    Inserting,
    Editing(Option<usize>),
}

pub enum Screen {
    WordInput(InputMode),
    List,
    SaveLocationInput,
}

pub struct SplitApp {
    tx: Sender<Event>,
    rx: Receiver<Event>,

    pub message: Option<String>,

    pub autocomplete: &'static str,
    pub input: String,
    pub screen: Screen,
    pub mnemonic: StatefulList<String>,
    pub should_quit: bool,

    pub phrases: [Vec<String>; 5],

    pub save_location: String,
}

impl SplitApp {
    pub fn new(tx: Sender<Event>, rx: Receiver<Event>) -> Self {
        Self {
            tx,
            rx,
            message: None,
            autocomplete: English::get_word(0).unwrap(),
            input: String::new(),
            screen: Screen::WordInput(InputMode::Normal),
            mnemonic: StatefulList::new(),
            phrases: empty_phrases(),
            should_quit: false,
            save_location: dirs::home_dir()
                .as_ref()
                .map(|path_buf| path_buf.to_string_lossy())
                .unwrap_or_else(|| Cow::Borrowed("/"))
                .to_string(),
        }
    }

    pub fn start_event_loop(&mut self, mut terminal: Term) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| view::draw(self, f))?;

            match self.rx.recv()? {
                Event::Input(event) => match self.screen {
                    Screen::WordInput(InputMode::Normal) => self.update_input_in_normal(event),
                    Screen::WordInput(InputMode::Inserting) => {
                        self.update_input_in_editing(event, None)
                    }
                    Screen::WordInput(InputMode::Editing(edit)) => {
                        self.update_input_in_editing(event, edit)
                    }
                    Screen::List => self.update_in_list(event),
                    Screen::SaveLocationInput => self.update_in_save_location(event),
                },
                Event::Effect(Effect::ReceivedPhrases(phrases)) => {
                    for (index, phrase) in phrases.iter().enumerate() {
                        let phrase_vec = phrase
                            .split(' ')
                            .map(ToString::to_string)
                            .collect::<Vec<String>>();
                        self.phrases[index] = phrase_vec
                    }
                }
                Event::Effect(Effect::ReceivedErrorMessage(msg)) => self.message = Some(msg),
                Event::Tick => {
                    if self.message.is_some() {
                        self.message = None;
                    }
                }
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

    fn update_input_in_editing(&mut self, key_event: KeyEvent, edit: Option<usize>) {
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
                        self.add_word_to_mnemonic(only_one.to_string(), edit);
                        self.input = "".to_string();
                    }
                    [head, ..] => self.autocomplete = head,
                }
            }
            KeyCode::Esc => self.screen = Screen::WordInput(InputMode::Normal),
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
                self.add_word_to_mnemonic(self.autocomplete.to_string(), edit);
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
            KeyCode::Char('i') => self.screen = Screen::WordInput(InputMode::Inserting),
            KeyCode::Esc => self.screen = Screen::WordInput(InputMode::Normal),
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
                self.phrases = empty_phrases();
                self.mnemonic.unselect();
                self.screen = Screen::WordInput(InputMode::Inserting)
            }
            KeyCode::Char('e') => {
                let current = self.mnemonic.selected();
                self.phrases = empty_phrases();
                self.mnemonic.unselect();
                self.screen = Screen::WordInput(InputMode::Editing(current))
            }
            KeyCode::Esc | KeyCode::Tab => {
                self.mnemonic.unselect();
                self.screen = Screen::WordInput(InputMode::Normal)
            }
            KeyCode::Up if key_event.modifiers.contains(KeyModifiers::ALT) => {
                self.mnemonic.move_up();
            }

            KeyCode::Enter if self.mnemonic.len() == 24 => {
                let mnemonic_code = self.mnemonic.items.join(" ");
                match splitmonic::get_split_phrases(mnemonic_code) {
                    Ok(phrases) => self
                        .tx
                        .send(Event::Effect(Effect::ReceivedPhrases(phrases)))
                        .expect("should always send"),
                    Err(err) => self
                        .tx
                        .send(Event::Effect(Effect::ReceivedErrorMessage(err.to_string())))
                        .expect("should always send"),
                }
            }

            KeyCode::Up => {
                if self.mnemonic.items.is_empty() {
                    self.mnemonic.unselect();
                    self.screen = Screen::WordInput(InputMode::Normal)
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

    fn update_in_save_location(&mut self, key_event: KeyEvent) {
        match key_event.code {
            _ => self.screen = Screen::List,
        }
    }

    fn add_word_to_mnemonic(&mut self, word: String, place: Option<usize>) {
        match (place, self.mnemonic.len()) {
            (None, 24) => {
                self.mnemonic.pop();
                self.mnemonic.push(word);
                self.screen = Screen::List
            }
            (None, len) => {
                self.mnemonic.push(word);
                if len == 23 {
                    self.screen = Screen::List
                }
            }
            (Some(index), len) => {
                self.mnemonic.items[index] = word;
                if len == 24 {
                    self.screen = Screen::List
                }
            }
        }
    }
}

fn empty_phrases() -> [Vec<String>; 5] {
    [
        Vec::with_capacity(28),
        Vec::with_capacity(28),
        Vec::with_capacity(28),
        Vec::with_capacity(28),
        Vec::with_capacity(28),
    ]
}
