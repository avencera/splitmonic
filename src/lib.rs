pub mod shamir;
mod wordlist;

use std::str::FromStr;

use bip39::Mnemonic;
use wordlist::{English, Wordlist};

pub fn share_to_phrase(mut share: Vec<u8>) -> String {
    let id = share.remove(0);
    let id_word = English::get_word(id as usize).unwrap();

    let words = Mnemonic::from_entropy(&share).unwrap().to_string();

    format!("{} {}", id_word, words)
}

pub fn phrase_to_share(phrase: String) -> Vec<u8> {
    let mut words: Vec<&str> = phrase.split(' ').collect();
    let id_word = words.remove(0);
    let id = English::get_index(id_word).unwrap();

    let mut share = Mnemonic::from_str(&words.join(" ")).unwrap().to_entropy();

    share.insert(0, id as u8);
    share
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

        let sh1_nu = share_to_phrase(share_1);
        let sh2_nu = share_to_phrase(share_2);
        let sh3_nu = share_to_phrase(share_3);

        let sh1_en = phrase_to_share(sh1_nu);
        let sh2_en = phrase_to_share(sh2_nu);
        let sh3_en = phrase_to_share(sh3_nu);

        let recovered = SecretData::recover_secret(3, vec![sh1_en, sh2_en, sh3_en]).unwrap();
        let recovered_nu = Mnemonic::from_entropy(&recovered).unwrap().to_string();

        assert_eq!(&recovered_nu, data)
    }
}
