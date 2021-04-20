pub mod shamir;
pub mod wordlist;

use crate::shamir::SecretData;
use bip39::Mnemonic;
use wordlist::{Wordlist, WordlistError};
use zeroize::Zeroize;

use thiserror::Error;
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error(transparent)]
    Wordlist(#[from] WordlistError),

    #[error(transparent)]
    BIP39(#[from] bip39::Error),

    #[error(transparent)]
    Shamir(#[from] shamir::ShamirError),

    #[error("error converting share(s) to phrase")]
    ShareToPhrase,

    #[error("not enough shares, gave {gave:?}, expected {expected:?}")]
    NotEnoughShares { gave: usize, expected: u8 },

    #[error("unable to recover secret")]
    UnableToRecoverSecret,

    #[error("all phrases must be from the same set")]
    MismatchedSet,
}

/// When given a BIP39 mnemonic code, returns a vec containing 5 split phrases.
/// 3 of these 5 codes can later be used to recreate your original mnemonic code.
pub fn get_split_phrases(mnemonic_code: String) -> Result<Vec<String>, Error> {
    use rand::Rng;

    let mut rng = rand::thread_rng();

    let mut shares = split::get_split_shares(mnemonic_code)?;

    let phrases = shares
        .iter_mut()
        .map(split::share_to_phrase)
        .collect::<Result<Vec<String>, Error>>()?;

    if shares.len() != phrases.len() {
        return Err(Error::ShareToPhrase);
    }

    // the first three words of all the phrases for this set are the same
    // the helps identify which set it belongs to
    let three_word_set_id = vec![
        rng.gen_range(0..2048),
        rng.gen_range(0..2048),
        rng.gen_range(0..2048),
    ]
    .iter()
    .map(|id| wordlist::English::get_word(*id as usize).unwrap())
    .collect::<Vec<String>>()
    .join(" ");

    let mut complete_phrases = Vec::with_capacity(5);
    for phrase in phrases {
        complete_phrases.push(format!("{} {}", &three_word_set_id, phrase))
    }

    Ok(complete_phrases)
}

/// When given a vector of at least 3 split phrases, returns the original mnemonic code
pub fn recover_mnemonic_code(mut split_phrases: Vec<String>) -> Result<String, Error> {
    let number_of_split_phrases = split_phrases.len();

    if number_of_split_phrases < 3 {
        return Err(Error::NotEnoughShares {
            gave: number_of_split_phrases,
            expected: 3,
        });
    }

    let split_phrases_words = split_phrases_into_words(&split_phrases);
    let split_phrases_without_set_ids = recover::verify_and_remove_set_id(split_phrases_words)?;

    let split_shares = split_phrases_without_set_ids
        .into_iter()
        .map(recover::words_to_share)
        .collect::<Result<Vec<Vec<u8>>, Error>>()?;
    split_phrases.zeroize();

    if split_shares.len() != number_of_split_phrases {
        return Err(Error::UnableToRecoverSecret);
    }

    let mut recovered =
        SecretData::recover_secret(3, split_shares).ok_or(Error::UnableToRecoverSecret)?;

    let mnemonic = Mnemonic::from_entropy(&recovered)?.to_string();
    recovered.zeroize();

    Ok(mnemonic)
}

mod split {
    //! Contains helper functions used for splitting the mnemonic code into phrases

    use crate::wordlist::{English, Wordlist};
    use crate::{shamir::SecretData, Error};
    use bip39::Mnemonic;
    use zeroize::Zeroize;

    pub(crate) fn get_split_shares(mut mnemonic_code: String) -> Result<[Vec<u8>; 5], Error> {
        let mut mnemonic = Mnemonic::parse(&mnemonic_code).unwrap();
        mnemonic_code.zeroize();

        let mut entropy = mnemonic.to_entropy();
        mnemonic.zeroize();

        let secret_data = SecretData::with_secret(&entropy, 3);
        entropy.zeroize();

        Ok([
            secret_data.get_share(1)?,
            secret_data.get_share(2)?,
            secret_data.get_share(3)?,
            secret_data.get_share(4)?,
            secret_data.get_share(5)?,
        ])
    }

    pub(crate) fn share_to_phrase(share: &mut Vec<u8>) -> Result<String, Error> {
        let id = share.remove(0);
        let id_word = English::get_word(id as usize)?;

        let words = Mnemonic::from_entropy(&share).unwrap().to_string();
        share.zeroize();

        Ok(format!("{} {}", id_word, words))
    }
}

mod recover {
    //! Contains helper functions used for recovering the mnemonic code from the split phrases

    use crate::{
        wordlist::{English, Wordlist},
        Error,
    };
    use bip39::{Language, Mnemonic};

    // verifies that all the phrases passed in are from the same set
    // if they are from the same set, returns the phrase without the set id words
    pub(crate) fn verify_and_remove_set_id(
        split_phrases: Vec<Vec<&str>>,
    ) -> Result<Vec<Vec<&str>>, Error> {
        let mut set_id = Vec::with_capacity(3);
        let mut without_ids = Vec::with_capacity(split_phrases.len());

        for split_phrase in split_phrases {
            if set_id.len() == 0 {
                set_id = split_phrase[0..3].into_iter().cloned().collect()
            }

            if set_id[0..3] != split_phrase[0..3] {
                return Err(Error::MismatchedSet);
            }

            without_ids.push(split_phrase[3..].into_iter().cloned().collect())
        }

        Ok(without_ids)
    }

    pub(crate) fn words_to_share(mut words: Vec<&str>) -> Result<Vec<u8>, Error> {
        let id_word = words.remove(0);
        let id = English::get_index(&id_word)?;

        let mut share = Mnemonic::parse_in(Language::English, &words.join(" "))?.to_entropy();

        share.insert(0, id as u8);

        Ok(share)
    }
}

// takes a vector of phrases and turns it into a vector of vector of words
// ```rust, ignore
// let phrases = vec!["hello there".to_string(), "how are you".to_string()];
// let words_vector = split_phrases_into_words(&phrases);
//
// assert_eq!(words_vector, vec![
//   vec!["hello", "there"],
//   vec!["how", "are", "you"]
// ])
// ```
fn split_phrases_into_words(split_phrases: &Vec<String>) -> Vec<Vec<&str>> {
    split_phrases
        .into_iter()
        .map(|phrase| phrase.split(' ').collect::<Vec<&str>>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;

    #[test]
    fn each_recovery_phrase_is_28_words() {
        let mnemonic_code = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point";
        let split_phrases = get_split_phrases(mnemonic_code.to_string()).unwrap();

        for split_phrase in split_phrases {
            assert_eq!(split_phrase.split(' ').collect::<Vec<&str>>().len(), 28)
        }
    }

    #[test]
    fn first_3_words_are_always_the_same() {
        let mnemonic_code = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point";
        let split_phrases = get_split_phrases(mnemonic_code.to_string()).unwrap();

        let three_word_id: Vec<String> = split_phrases[0]
            .split(' ')
            .collect::<Vec<&str>>()
            .as_slice()[0..3]
            .iter()
            .map(ToString::to_string)
            .collect();

        for split_phrase in split_phrases {
            assert_eq!(
                split_phrase.split(' ').collect::<Vec<&str>>().as_slice()[0..3],
                three_word_id
            )
        }
    }

    #[test]
    fn split_and_recover() {
        let mut rng = rand::thread_rng();

        let mnemonic_code = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point";
        let mut split_phrases = get_split_phrases(mnemonic_code.to_string()).unwrap();

        split_phrases.shuffle(&mut rng);

        split_phrases.pop();
        split_phrases.pop();

        let recovered_mnemonic = recover_mnemonic_code(split_phrases).unwrap();

        assert_eq!(recovered_mnemonic, mnemonic_code.to_string())
    }
}
