use splitmonic::wordlist::english::English;
use splitmonic::wordlist::Wordlist;

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
