/// Taken from: https://github.com/summa-tx/bitcoins-rs/tree/main/bip39/src/wordlist
/// and modified to make looks a bit quicker
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
    fn get_word(index: usize) -> Result<String, WordlistError> {
        Self::WORDLIST
            .words
            .get(&index)
            .ok_or_else(|| WordlistError::InvalidIndex(index))
            .map(|word| word.to_string())
    }

    /// Returns the index of a given word from the word list.
    fn get_index(word: &str) -> Result<usize, WordlistError> {
        Self::WORDLIST
            .indexes
            .get(word)
            .ok_or_else(|| WordlistError::InvalidWord(word.into()))
            .map(|usize| *usize)
    }

    /// Returns the word list as a string.
    fn get_all() -> Vec<&'static str> {
        Self::WORDLIST.words.values().cloned().collect()
    }
}
