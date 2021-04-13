use bip39::Mnemonic;
use splitnox::shamir::SecretData;

fn main() {
    let data = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point";

    let nu = Mnemonic::parse(data).unwrap();
    let entropy = nu.to_entropy();

    let secret_data = SecretData::with_secret(&entropy, 3);

    let share_1 = secret_data.get_share(1).unwrap();
    let share_2 = secret_data.get_share(2).unwrap();
    let share_3 = secret_data.get_share(3).unwrap();
    let share_4 = secret_data.get_share(4).unwrap();
    let share_5 = secret_data.get_share(5).unwrap();

    let sh1_nu = splitnox::share_to_phrase(share_1).unwrap();
    let sh2_nu = splitnox::share_to_phrase(share_2).unwrap();
    let sh3_nu = splitnox::share_to_phrase(share_3).unwrap();
    let sh4_nu = splitnox::share_to_phrase(share_4).unwrap();
    let sh5_nu = splitnox::share_to_phrase(share_5).unwrap();

    println!("PHRASE 1: {}", sh1_nu);
    println!("PHRASE 2: {}", sh2_nu);
    println!("PHRASE 3: {}", sh3_nu);
    println!("PHRASE 4: {}", sh4_nu);
    println!("PHRASE 5: {}", sh5_nu);

    let sh1_en = splitnox::phrase_to_share(sh1_nu).unwrap();
    let sh2_en = splitnox::phrase_to_share(sh4_nu).unwrap();
    let sh3_en = splitnox::phrase_to_share(sh2_nu).unwrap();

    let recovered = SecretData::recover_secret(3, vec![sh1_en, sh2_en, sh3_en]).unwrap();
    let recovered_nu = Mnemonic::from_entropy(&recovered);

    println!("Recovered: {}", recovered_nu.unwrap().to_string());
}
