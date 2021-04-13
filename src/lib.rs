pub mod shamir;
mod wordlist;

use bip39::{Language, Mnemonic};
use wordlist::{English, Wordlist, WordlistError};

use thiserror::Error;
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error(transparent)]
    Wordlist(#[from] WordlistError),

    #[error(transparent)]
    BIP39(#[from] bip39::Error),
}

pub fn share_to_phrase(mut share: Vec<u8>) -> Result<String, Error> {
    let id = share.remove(0);
    let id_word = English::get_word(id as usize)?;

    let words = Mnemonic::from_entropy(&share).unwrap().to_string();

    Ok(format!("{} {}", id_word, words))
}

pub fn phrase_to_share(phrase: String) -> Result<Vec<u8>, Error> {
    let mut words: Vec<&str> = phrase.split(' ').collect();
    let id_word = words.remove(0);
    let id = English::get_index(id_word)?;

    let mut share = Mnemonic::parse_in(Language::English, &words.join(" "))?.to_entropy();

    share.insert(0, id as u8);

    Ok(share)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shamir::SecretData;

    #[test]
    fn split_and_recover() {
        let data = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point";
        let nu = Mnemonic::parse(data).unwrap();
        let entropy = nu.to_entropy();

        let secret_data = SecretData::with_secret(&entropy, 3);

        let share_1 = secret_data.get_share(1).unwrap();
        let share_2 = secret_data.get_share(2).unwrap();
        let share_3 = secret_data.get_share(3).unwrap();

        let sh1_nu = share_to_phrase(share_1).unwrap();
        let sh2_nu = share_to_phrase(share_2).unwrap();
        let sh3_nu = share_to_phrase(share_3).unwrap();

        let sh1_en = phrase_to_share(sh1_nu).unwrap();
        let sh2_en = phrase_to_share(sh2_nu).unwrap();
        let sh3_en = phrase_to_share(sh3_nu).unwrap();

        let recovered = SecretData::recover_secret(3, vec![sh1_en, sh2_en, sh3_en]).unwrap();
        let recovered_nu = Mnemonic::from_entropy(&recovered).unwrap().to_string();

        assert_eq!(&recovered_nu, data)
    }
}
