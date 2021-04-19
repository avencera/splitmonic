fn main() {
    let mnemonic_code = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point".to_string();
    let mut split_phrases = splitmonic::get_split_phrases(mnemonic_code).unwrap();

    for (index, share_phrase) in split_phrases.iter().enumerate() {
        println!("PHRASE {}: {}", index + 1, share_phrase);
    }

    split_phrases.pop();
    split_phrases.pop();

    let recovered_mnemonic = splitmonic::recover_mnemonic_code(split_phrases);

    println!("\nRecovered: {}", recovered_mnemonic.unwrap().to_string());
}
