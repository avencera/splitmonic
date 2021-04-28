use once_cell::unsync::Lazy;

use super::WordlistData;
use crate::wordlist::Wordlist;

/// The list of words as supported in the English language.
pub const ENGLISH: &str = include_str!("./words/english.txt");

#[derive(Clone, Debug, PartialEq)]
/// The English wordlist that implements the Wordlist trait.
pub struct English;

impl Wordlist for English {
    const WORDLIST: Lazy<WordlistData> = Lazy::new(|| {
        let words = ENGLISH.lines().enumerate().collect();
        let indexes = ENGLISH
            .lines()
            .enumerate()
            .map(|(index, word)| (word, index))
            .collect();

        WordlistData { words, indexes }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::wordlist::WordlistError;

    #[test]
    fn test_get() {
        assert_eq!(English::get_word(3), Ok("about"));
        assert_eq!(English::get_word(2044), Ok("zebra"));
        assert_eq!(
            English::get_word(2048),
            Err(WordlistError::InvalidIndex(2048))
        );
    }

    #[test]
    fn test_get_index() {
        assert_eq!(English::get_index("about"), Ok(3));
        assert_eq!(English::get_index("zebra"), Ok(2044));
        assert_eq!(
            English::get_index("somerandomword"),
            Err(WordlistError::InvalidWord("somerandomword".to_string()))
        );
    }

    #[test]
    fn test_get_all() {
        assert_eq!(English::get_all().len(), 2048);
    }
}
