use thiserror::Error;

use crate::wordlist::{English, Wordlist};

#[derive(Debug, Error, PartialEq, Clone)]
pub enum Error {
    #[error("this mnemonic length is invalid, expected {expected:?}, found: {given:?}\nmnemonic: {mnemonic:?}")]
    MnemonicLength {
        expected: usize,
        given: usize,
        mnemonic: String,
    },

    #[error("invalid words found, invalid word indexes: {indexes:?},\ninvalid words: {invalid_words:?}\n given_phrase: {given_phrase:?}")]
    Words {
        indexes: Vec<usize>,
        invalid_words: Vec<String>,
        given_phrase: String,
    },

    #[error("invalid number of split phrases, expected: {expected:?}, found: {given:?}, all phrases: {all_phrases:?}")]
    PhrasesLengthThreshold {
        expected: usize,
        given: usize,
        all_phrases: String,
    },

    #[error("found invalid split phrase lengths, the following phrases weren't long enough: {invalid_phrases:?}\n\
    they were expected to all be 28 words long. Instead they were of lengths: {invalid_phrase_lengths:?}\n\
    all phrases: {all_phrases:?}")]
    PhraseLength {
        invalid_phrase_lengths: Vec<usize>,
        invalid_phrases: Vec<String>,
        all_phrases: String,
    },

    #[error("invalid words in split phrases: {0:?}")]
    InvalidSplitPhraseWords(Vec<(usize, Error)>),

    #[error("mismatched set(s), expected: {expected:?}, found: {given:?}")]
    MismatchedSet {
        expected: String,
        given: Vec<(usize, String)>,
    },
}

pub fn validate_mnemonic_code(mnemonic: String) -> Result<(), Error> {
    let mnemonic_vec: Vec<&str> = mnemonic.split(' ').collect();

    if mnemonic_vec.len() != 24 {
        Err(Error::MnemonicLength {
            expected: 24,
            given: mnemonic_vec.len(),
            mnemonic: mnemonic.clone(),
        })?
    }

    validate_all_correct_words(&mnemonic_vec)?;

    Ok(())
}

pub fn validate_split_phrases(split_phrases: Vec<String>) -> Result<(), Error> {
    if split_phrases.len() != 3 {
        Err(Error::PhrasesLengthThreshold {
            expected: 3,
            given: split_phrases.len(),
            all_phrases: split_phrases.join("\n"),
        })?
    }

    let split_phrases_vec: Vec<Vec<&str>> = split_phrases
        .iter()
        .map(|phrase| phrase.split(' ').collect())
        .collect();

    validate_lengths_of_phrases(&split_phrases_vec)?;
    validate_words_in_phrases(&split_phrases_vec)?;
    validate_part_of_same_set(&split_phrases_vec)?;

    Ok(())
}

fn validate_all_correct_words(mnemonic_vec: &[&str]) -> Result<(), Error> {
    let mut indexes = vec![];
    let mut invalid_words = vec![];

    for (index, word) in mnemonic_vec.iter().enumerate() {
        if let Err(_) = English::get_index(word) {
            indexes.push(index);
            invalid_words.push(word.to_string());
        }
    }

    if !indexes.is_empty() {
        Err(Error::Words {
            indexes,
            invalid_words,
            given_phrase: mnemonic_vec.join(" "),
        })?
    }

    Ok(())
}

fn validate_lengths_of_phrases(split_phrases: &Vec<Vec<&str>>) -> Result<(), Error> {
    let mut invalid_phrase_lengths = vec![];
    let mut invalid_phrases = vec![];

    for phrases in split_phrases {
        if phrases.len() != 28 {
            invalid_phrases.push(phrases.join(" "));
            invalid_phrase_lengths.push(phrases.len());
        }
    }

    if !invalid_phrases.is_empty() {
        Err(Error::PhraseLength {
            invalid_phrase_lengths,
            invalid_phrases,
            all_phrases: split_phrases
                .iter()
                .cloned()
                .map(|phrases| phrases.join(" "))
                .collect::<Vec<String>>()
                .join("\n"),
        })?
    }

    Ok(())
}

fn validate_words_in_phrases(split_phrases: &Vec<Vec<&str>>) -> Result<(), Error> {
    let mut invalid_words: Vec<(usize, Error)> = vec![];

    for (index, phrases) in split_phrases.iter().enumerate() {
        if let Err(error) = validate_all_correct_words(phrases) {
            invalid_words.push((index, error))
        }
    }

    if !invalid_words.is_empty() {
        Err(Error::InvalidSplitPhraseWords(invalid_words))?
    }

    Ok(())
}

fn validate_part_of_same_set(split_phrases: &Vec<Vec<&str>>) -> Result<(), Error> {
    let mut set_id = Vec::with_capacity(3);
    let mut mismatched_sets = vec![];

    for (index, split_phrase) in split_phrases.iter().enumerate() {
        if set_id.is_empty() {
            set_id = split_phrase[0..3].to_vec()
        }

        if set_id[0..3] != split_phrase[0..3] {
            mismatched_sets.push((index, split_phrase[0..3].join(" ")))
        }
    }

    if !mismatched_sets.is_empty() {
        Err(Error::MismatchedSet {
            given: mismatched_sets,
            expected: set_id.join(" "),
        })?
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_error_on_wrong_length() {
        let error = validate_mnemonic_code("this is a fail".to_string()).unwrap_err();

        assert_eq!(
            error.clone(),
            Error::MnemonicLength {
                expected: 24,
                given: 4,
                mnemonic: "this is a fail".to_string()
            }
        );

        assert_eq!(
            error.to_string(),
            "this mnemonic length is invalid, expected 24, found: 4\nmnemonic: \"this is a fail\""
        )
    }

    #[test]
    fn produces_error_on_wrong_words() {
        let mnemonic = "abandon abandon abandon abandon ford abandon abandon abandon abandon abandan abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon f150 art".to_string();
        let error = validate_mnemonic_code(mnemonic.clone()).unwrap_err();

        assert_eq!(
            error,
            Error::Words {
                indexes: vec![4, 9, 22],
                invalid_words: vec![
                    "ford".to_string(),
                    "abandan".to_string(),
                    "f150".to_string()
                ],
                given_phrase: mnemonic.clone()
            }
        );
    }

    #[test]
    fn produces_error_when_not_enough_phrases() {
        let phrases = vec![
            "hello this is my first phrase".to_string(),
            "this is my second phrase".to_string(),
        ];
        let error = validate_split_phrases(phrases).unwrap_err();

        assert_eq!(
            error,
            Error::PhrasesLengthThreshold {
                expected: 3,
                given: 2,
                all_phrases: "hello this is my first phrase\nthis is my second phrase".to_string(),
            }
        )
    }

    #[test]
    fn produces_error_when_phrases_are_not_long_enough() {
        let phrases = vec![
            "hello this is my first phrase".to_string(),
            "this is my second phrase".to_string(),
            "third phrase".to_string(),
        ];

        let error = validate_split_phrases(phrases.clone()).unwrap_err();

        assert_eq!(
            error,
            Error::PhraseLength {
                invalid_phrase_lengths: vec![6, 5, 2],
                invalid_phrases: phrases.clone(),
                all_phrases:
                    "hello this is my first phrase\nthis is my second phrase\nthird phrase"
                        .to_string(),
            },
        )
    }

    #[test]
    fn test_validate_part_of_same_set() {
        let phrases = vec![
            "hello hello hello some other random stuff",
            "hello hello hello more random stuff",
            "hello bad hello even more random stuff",
        ]
        .iter()
        .map(|phrase| phrase.split(" ").collect())
        .collect();

        let error = validate_part_of_same_set(&phrases).unwrap_err();

        assert_eq!(
            error,
            Error::MismatchedSet {
                given: vec![(2, "hello bad hello".to_string())],
                expected: "hello hello hello".to_string(),
            },
        )
    }
}
