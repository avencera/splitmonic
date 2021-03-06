mod split_app;
mod ui;

use crate::split_app::SplitApp;
use crossbeam_channel::unbounded;
use crossterm::{
    event::{self, Event as CEvent},
    execute, terminal,
};
use eyre::{Context, Result};
use std::{
    io::{self, Stdout},
    thread,
    time::{Duration, Instant},
};
use structopt::clap::AppSettings;
use tui::{backend::CrosstermBackend, Terminal};

pub type Backend = CrosstermBackend<Stdout>;
pub type Term = Terminal<Backend>;

use structopt::StructOpt;

/// Split your BIP39 mnemonic phrase using shamir secret sharing
#[derive(StructOpt, Debug)]
#[structopt(name = "splitmonic", global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp])]
enum Splitmonic {
    #[structopt(
        name = "split",
        about = "Split you're mnemonic into multiple split phrases"
    )]
    Split {
        #[structopt(short, long, help = "use the interactive TUI")]
        interactive: bool,

        #[structopt(
            short,
            long,
            help = "your mnemonic",
            required_unless = "interactive",
            conflicts_with = "interactive"
        )]
        mnemonic: Option<String>,
    },
    #[structopt(
        name = "combine",
        about = "Combine you're split phrases into your original mnemonic"
    )]
    Combine {
        #[structopt(short, long, help = "use the interactive TUI")]
        interactive: bool,

        #[structopt(
            short="s",
            long,
            help = "3 of 5 split phrases",
            required_unless_one = &["split-phrases-1", "split-phrases-2", "split-phrases-3", "interactive", "split-phrase-files"],
            conflicts_with = "interactive",
            use_delimiter = true,
            min_values = 3,
            max_values = 3
        )]
        all_split_phrases: Option<Vec<String>>,

        #[structopt(short="f", long, 
        help = "list of files containing your split phrases",             
        required_unless_one = &["split-phrases-1", "split-phrases-2", "split-phrases-3", "interactive", "all-split-phrases"],
        conflicts_with = "interactive",
        use_delimiter = true,
        min_values = 1,
        max_values = 3
        )]
        split_phrase_files: Option<Vec<String>>,

        #[structopt(
            short = "1",
            visible_alias = "sp1",
            long,
            help = "first split phrase",
            conflicts_with = "interactive",
            use_delimiter = true,
            min_values = 28,
            max_values = 28
        )]
        split_phrases_1: Option<Vec<String>>,

        #[structopt(
            short = "2",
            visible_alias = "sp2",
            long,
            help = "second split phrase",
            conflicts_with = "interactive",
            use_delimiter = true,
            min_values = 28,
            max_values = 28
        )]
        split_phrases_2: Option<Vec<String>>,

        #[structopt(
            short = "3",
            visible_alias = "sp3",
            long,
            help = "third split phrase",
            conflicts_with = "interactive",
            use_delimiter = true,
            min_values = 28,
            max_values = 28
        )]
        split_phrases_3: Option<Vec<String>>,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let opt = Splitmonic::from_args();

    match opt {
        Splitmonic::Split {
            interactive: true, ..
        } => setup_split_tui(),

        Splitmonic::Split {
            interactive: false,
            mnemonic: Some(mnemonic),
            ..
        } => {
            match get_split_phrases(mnemonic) {
                Ok(split_phrases) => {
                    for (index, phrase) in split_phrases.iter().enumerate() {
                        println!("\n######################################################");
                        println!(
                            "############## Split Phrase {} of 5 ###################",
                            index + 1
                        );
                        println!("######################################################");

                        phrase
                            .split(' ')
                            .enumerate()
                            .for_each(|(index, word)| println!("{}: {}", index + 1, word));

                        println!();
                    }
                }
                Err(error) => eprintln!("Error splitting mnemonic into split phrases: {}", error),
            }

            Ok(())
        }

        Splitmonic::Combine {
            interactive: true, ..
        } => Ok(()),

        splitmonic @ Splitmonic::Combine {
            interactive: false, ..
        } => {
            let mnemonic_code = get_mnemonic_code_from_combine_cli(splitmonic);

            match mnemonic_code {
                Ok(mnemonic_code) => {
                    println!("\nSuccessfully recovered your mnemonic code:\n");
                    for (index, word) in mnemonic_code.split(' ').enumerate() {
                        println!("{}: {}", index + 1, word)
                    }
                }
                Err(error) => {
                    eprintln!("Error combining split phrases: {}", error)
                }
            }

            Ok(())
        }

        // any other combinations are impossible
        _ => Err(eyre::eyre!("unreachable")),
    }
}

fn get_split_phrases(mnemonic: String) -> Result<Vec<String>> {
    splitmonic::validation::validate_mnemonic_code(&mnemonic)?;
    Ok(splitmonic::get_split_phrases(mnemonic)?)
}

fn get_mnemonic_code_from_combine_cli(splitmonic: Splitmonic) -> Result<String> {
    match splitmonic {
        Splitmonic::Combine {
            all_split_phrases: Some(split_phrases),
            ..
        } => {
            let split_phrases: Vec<String> = split_phrases
                .iter()
                .map(|phrase| phrase.trim().to_string())
                .collect();

            splitmonic::validation::validate_split_phrases(split_phrases.clone())?;

            Ok(splitmonic::recover_mnemonic_code(split_phrases)?)
        }

        Splitmonic::Combine {
            split_phrase_files: Some(ref file_paths),
            split_phrases_1,
            split_phrases_2,
            split_phrases_3,
            ..
        } => {
            let split_phrases = get_split_phrases_from_files(
                file_paths,
                vec![split_phrases_1, split_phrases_2, split_phrases_3],
            );
            splitmonic::validation::validate_split_phrases(split_phrases.clone())?;

            Ok(splitmonic::recover_mnemonic_code(split_phrases)?)
        }

        Splitmonic::Combine {
            split_phrases_1,
            split_phrases_2,
            split_phrases_3,
            ..
        } => {
            let split_phrases = vec![split_phrases_1, split_phrases_2, split_phrases_3]
                .iter()
                .filter_map(|phrase| phrase.as_ref())
                .map(|phrase| clean_and_combine_phrase(phrase))
                .collect::<Vec<String>>();

            splitmonic::validation::validate_split_phrases(split_phrases.clone())?;

            Ok(splitmonic::recover_mnemonic_code(split_phrases)?)
        }

        // any other combinations are impossible
        _ => Err(eyre::eyre!("unreachable")),
    }
}

fn clean_and_combine_phrase(phrase: &[String]) -> String {
    phrase
        .iter()
        .map(|phrase| phrase.trim())
        .filter(|phrase| !phrase.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}

fn get_split_phrases_from_files(
    file_paths: &[String],
    phrases_direct: Vec<Option<Vec<String>>>,
) -> Vec<String> {
    let phrases_from_files = file_paths
        .iter()
        .map(|file| read_and_get_phrases_from_file(file))
        .filter_map(Result::ok)
        .map(|phrase| clean_and_combine_phrase(&phrase));

    let phrases_direct = phrases_direct
        .iter()
        .filter_map(|phrase| phrase.as_ref())
        .map(|phrase| clean_and_combine_phrase(phrase));

    phrases_from_files.chain(phrases_direct).collect()
}

fn read_and_get_phrases_from_file(path: &str) -> Result<Vec<String>> {
    let file_contents =
        std::fs::read_to_string(path).wrap_err_with(|| format!("Unable to read file: {}", path))?;

    let words = extracts_words_from_file_contents(file_contents);

    Ok(words)
}

fn extracts_words_from_file_contents(file_contents: String) -> Vec<String> {
    let mut words = Vec::with_capacity(28);

    for line in file_contents.lines() {
        let word: String = line.chars().filter(|char| char.is_alphabetic()).collect();

        if !word.is_empty() {
            words.push(word)
        }
    }

    words
}

fn setup_split_tui() -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    const MNEMONIC_CODE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    #[test]
    fn combines_using_all_phrases_option() {
        let splitmonic = Splitmonic::from_iter(&["splitmonic", "combine", 
        "-s=embody fog drop ability sword volume hat detail blue pride yard benefit coach primary now pledge head panel hour congress curtain plug over ordinary debris release tent coin,embody fog drop able network accident hedgehog sibling toilet outdoor quick subway hurdle picture property false quit notable panther crucial already supply mother beef recycle spell rich enhance,embody fog drop about embrace visa adapt winner wine dash fabric snack drip auction deputy visit shift animal various bread country lecture assist marriage merit goat gravity glove"
        ]);

        let mnemonic_code = get_mnemonic_code_from_combine_cli(splitmonic).unwrap();

        assert_eq!(&mnemonic_code, MNEMONIC_CODE);
    }

    #[test]
    fn combines_using_phrases_passed_in_separately() {
        let splitmonic = Splitmonic::from_iter(&["splitmonic", "combine", 
        "--sp1=embody, fog, drop, ability, sword, volume, hat,   detail, blue, pride, yard, benefit, coach, primary, now, pledge, head, panel, hour, congress, curtain, plug, over, ordinary, debris, release, tent, coin", 
        "--sp2=embody, fog, drop, about, embrace, visa, adapt, winner, wine, dash, fabric, snack, drip, auction, deputy, visit, shift, animal, various, bread, country, lecture, assist, marriage, merit, goat, gravity, glove", 
        "--sp3=embody, fog, drop, able, network, accident, hedgehog, sibling, toilet, outdoor, quick, subway, hurdle, picture, property, false, quit, notable, panther, crucial, already, supply, mother, beef, recycle, spell, rich, enhance"]);

        let mnemonic_code = get_mnemonic_code_from_combine_cli(splitmonic).unwrap();

        assert_eq!(&mnemonic_code, MNEMONIC_CODE);
    }

    #[test]
    fn extracts_words_from_output_file_format() {
        let file_contents = "
        1: gun
        2: dismiss
        3: area
        4: ability
        5: laptop
        6: live
        7: ignore
        8: love
        9: ride
        10: deposit
        11: upset
        12: enemy
        13: start
        14: leopard
        15: domain
        16: exile
        17: talent
        18: enroll
        19: north
        20: position
        21: talk
        22: hope
        23: script
        24: parent
        25: tongue
        26: ride
        27: pepper
        28: brisk"
            .to_string();

        let words_list: Vec<String> = vec![
            "gun", "dismiss", "area", "ability", "laptop", "live", "ignore", "love", "ride",
            "deposit", "upset", "enemy", "start", "leopard", "domain", "exile", "talent", "enroll",
            "north", "position", "talk", "hope", "script", "parent", "tongue", "ride", "pepper",
            "brisk",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();

        assert_eq!(extracts_words_from_file_contents(file_contents), words_list)
    }
}
