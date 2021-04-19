pub mod shamir;
mod wordlist;

use crate::shamir::SecretData;
use bip39::{Language, Mnemonic};
use wordlist::{English, Wordlist, WordlistError};
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
    NotEnoughShares { gave: u8, expected: u8 },

    #[error("unable to recover secret")]
    UnableToRecoverSecret,
}

pub fn get_split_phrases(mnemonic_code: String) -> Result<Vec<String>, Error> {
    let mut shares = get_split_shares(mnemonic_code)?;

    let phrases: Vec<String> = shares
        .iter_mut()
        .map(share_to_phrase)
        .filter_map(Result::ok)
        .collect();

    if shares.len() == phrases.len() {
        Ok(phrases)
    } else {
        Err(Error::ShareToPhrase)
    }
}

pub fn recover_mnemonic_code(split_phrases: Vec<String>) -> Result<String, Error> {
    let number_of_split_phrases = split_phrases.len();

    if number_of_split_phrases < 3 {
        return Err(Error::NotEnoughShares {
            gave: number_of_split_phrases as u8,
            expected: 3,
        });
    }

    let split_shares: Vec<Vec<u8>> = split_phrases
        .into_iter()
        .map(phrase_to_share)
        .filter_map(Result::ok)
        .collect();

    if split_shares.len() != number_of_split_phrases {
        return Err(Error::UnableToRecoverSecret);
    }

    let mut recovered =
        SecretData::recover_secret(3, split_shares).ok_or(Error::UnableToRecoverSecret)?;

    let mnemonic = Mnemonic::from_entropy(&recovered)?.to_string();
    recovered.zeroize();

    Ok(mnemonic)
}

fn get_split_shares(mut mnemonic_code: String) -> Result<[Vec<u8>; 5], Error> {
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

fn share_to_phrase(share: &mut Vec<u8>) -> Result<String, Error> {
    let id = share.remove(0);
    let id_word = English::get_word(id as usize)?;

    let words = Mnemonic::from_entropy(&share).unwrap().to_string();
    share.zeroize();

    Ok(format!("{} {}", id_word, words))
}

fn phrase_to_share(mut phrase: String) -> Result<Vec<u8>, Error> {
    let mut words: Vec<&str> = phrase.split(' ').collect();

    let id_word = words.remove(0);
    let id = English::get_index(id_word)?;

    let mut share = Mnemonic::parse_in(Language::English, &words.join(" "))?.to_entropy();
    phrase.zeroize();

    share.insert(0, id as u8);

    Ok(share)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;

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
