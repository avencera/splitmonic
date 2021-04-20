//! Taken from: https://github.com/summa-tx/bitcoins-rs/tree/main/bip39/src/wordlist
//! and modified to make look ups a bit quicker, the trade of is it uses more memory
pub mod english;
pub use self::english::*;
use once_cell::unsync::Lazy;
use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
/// The error type returned while interacting with wordists.
pub enum WordlistError {
    /// Describes the error when the wordlist is queried at an invalid index.
    #[error("the index `{0}` is invalid")]
    InvalidIndex(usize),
    /// Describes the error when the wordlist does not contain the queried word.
    #[error("the word `{0}` is invalid")]
    InvalidWord(String),
}

#[derive(Debug)]
pub struct WordlistData {
    words: HashMap<usize, &'static str>,
    indexes: HashMap<&'static str, usize>,
}

// The Wordlist trait that every language's wordlist must implement.
pub trait Wordlist {
    const WORDLIST: Lazy<WordlistData>;

    /// Returns the word of a given index from the word list.
    fn get_word(index: usize) -> Result<&'static str, WordlistError> {
        Self::WORDLIST
            .words
            .get(&index)
            .ok_or(WordlistError::InvalidIndex(index))
            .map(|word| word.clone())
    }

    /// Returns the index of a given word from the word list.
    fn get_index(word: &str) -> Result<usize, WordlistError> {
        Self::WORDLIST
            .indexes
            .get(word)
            .ok_or_else(|| WordlistError::InvalidWord(word.into()))
            .map(|usize| *usize)
    }

    fn contains_word(word: &str) -> bool {
        Self::get_index(word).is_ok()
    }

    /// Returns the word list as a string.
    fn get_all() -> Vec<&'static str> {
        let mut words: Vec<&'static str> = Self::WORDLIST.words.values().cloned().collect();
        words.sort();
        words
    }

    fn starting_with(start: &str) -> Vec<&'static str> {
        let mut words = Self::WORDLIST
            .words
            .values()
            .into_iter()
            .filter(|word| word.starts_with(start))
            .cloned()
            .collect::<Vec<&'static str>>();

        words.sort();
        words
    }

    fn next_starting_with(start: &str, current_word: &str) -> Option<&'static str> {
        let words = Self::starting_with(start);
        let position = words.iter().position(|word| word == &current_word)?;

        // if the last word cycle back to the first word in the list
        let position = if position == (words.len() - 1) {
            0
        } else {
            position
        };

        Some(words.get(position + 1)?.clone())
    }
}
