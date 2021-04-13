mod shamir;
use bip39::{Language, Mnemonic};
use num_bigint::BigUint;
use shamir::SecretData;

fn main() {
    // let mut rng = rand::thread_rng();
    // let m = Mnemonic::generate_in_with(&mut rng, Language::English, 12).unwrap();

    // let entropy = m.to_entropy();
    // let string = m.clone().to_string();

    // println!("STRING: {}", string);

    let data = "dance monitor unveil wood cycle uphold video elephant run unlock theme year divide text lyrics captain expose garlic bundle patrol praise net hour point";

    let nu = Mnemonic::parse(data).unwrap();
    let entropy = nu.to_entropy();

    let secret_data = SecretData::with_secret(&entropy, 3);

    // let share1 = secret_data.get_share(1).unwrap();
    // let share2 = secret_data.get_share(2).unwrap();
    // let share3 = secret_data.get_share(3).unwrap();

    let share_1 = vec![
        1, 216, 249, 70, 189, 223, 27, 211, 217, 156, 10, 241, 208, 130, 228, 160, 157, 252, 233,
        202, 50, 2, 63, 85, 15, 190, 76, 165, 234, 114, 7, 71, 203,
    ];

    let share_2 = vec![
        2, 83, 162, 114, 137, 223, 77, 219, 109, 217, 152, 236, 180, 68, 228, 107, 1, 226, 101,
        199, 106, 182, 215, 16, 157, 108, 97, 201, 41, 225, 242, 120, 82,
    ];

    let share_3 = vec![
        3, 188, 10, 215, 141, 254, 197, 101, 105, 249, 96, 35, 217, 155, 195, 75, 227, 109, 115,
        178, 57, 237, 249, 21, 57, 42, 84, 60, 89, 10, 220, 100, 12,
    ];

    let share_1_bit_11 = to_u11(&share_1);
    let share_1_recovered = to_u8(&share_1_bit_11);

    println!("U11: {:#?}", share_1_bit_11);
    println!("U8: {:#?}", share_1_recovered.len());

    let share_2_bit_11 = to_u11(&share_2);
    let share_2_recovered = to_u8(&share_2_bit_11);

    println!("U11: {:#?}", share_2_bit_11);
    println!("U8: {:#?}", share_2_recovered.len());

    let share_3_bit_11 = to_u11(&share_3);
    let share_3_recovered = to_u8(&share_3_bit_11);

    println!("U11: {:#?}", share_3_bit_11);
    println!("U8: {:#?}", share_3_recovered.len());

    let recovered = SecretData::recover_secret(
        3,
        vec![share_1_recovered, share_2_recovered, share_3_recovered],
    )
    .unwrap();
    let recovered_nu = Mnemonic::from_entropy(&recovered);

    println!("Recovered: {}", recovered_nu.unwrap().to_string());
}

fn to_u11(digits: &[u8]) -> Vec<u16> {
    let mut bit_11 = vec![];

    let mut binary = String::new();

    for digit in digits {
        binary.push_str(&format!("{:b}", digit))
    }

    let binary: Vec<char> = binary.chars().into_iter().collect();

    for chunk in binary.chunks(11) {
        let code: String = chunk.iter().map(|c| c.to_string()).collect();
        let number = u16::from_str_radix(&code, 2).unwrap();
        bit_11.push(number)
    }

    bit_11
}

fn to_u8(digits: &[u16]) -> Vec<u8> {
    let mut bit8: Vec<u8> = vec![];

    let mut binary = String::new();

    for digit in digits {
        binary.push_str(&format!("{:b}", digit))
    }

    let binary: Vec<char> = binary.chars().into_iter().collect();

    for chunk in binary.chunks(8) {
        let code: String = chunk.iter().map(|c| c.to_string()).collect();
        let number = u8::from_str_radix(&code, 2).unwrap();
        bit8.push(number)
    }

    bit8
}
