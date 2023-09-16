use aes::{
    hazmat::{cipher_round as aes_enc, equiv_inv_cipher_round as aes_dec},
    Block,
};
use hex_literal::hex;

// key0, key1, key2, key3 = Hash512("RandomX AesGenerator1R keys")
const GENERATOR_1_KEY_0: [u8; 16] = hex!("53a5ac6d096671622b55b5db1749f4b4");
const GENERATOR_1_KEY_1: [u8; 16] = hex!("07af7c6d0d716a8478d325174edca10d");
const GENERATOR_1_KEY_2: [u8; 16] = hex!("f162123fc67e949f4f79c0f445e3203e");
const GENERATOR_1_KEY_3: [u8; 16] = hex!("3581ef6a7c31bab1884c311654911649");

// key0, key1, key2, key3 = Hash512("RandomX AesGenerator4R keys 0-3")
const GENERATOR_4_KEY_0: [u8; 16] = hex!("ddaa2164db3d83d12b6d542f3fd2e599");
const GENERATOR_4_KEY_1: [u8; 16] = hex!("50340eb2553f91b6539df706e5cddfa5");
const GENERATOR_4_KEY_2: [u8; 16] = hex!("04d93e5caf7b5e519f67a40abf021c17");
const GENERATOR_4_KEY_3: [u8; 16] = hex!("63376285085d8fe7853767cd91d2ded8");
// key4, key5, key6, key7 = Hash512("RandomX AesGenerator4R keys 4-7")
const GENERATOR_4_KEY_4: [u8; 16] = hex!("736f82b5a6a7d6e36d8b513db4ff9e22");
const GENERATOR_4_KEY_5: [u8; 16] = hex!("f36b56c7d9b3109c4e4d02e9d2b772b2");
const GENERATOR_4_KEY_6: [u8; 16] = hex!("e7c973f28ba365f70a66a92ba7ef3bf6");
const GENERATOR_4_KEY_7: [u8; 16] = hex!("09d67c7ade395891fdd1060c2d76b0c0");

// state0, state1, state2, state3 = Hash512("RandomX AesHash1R state")
const HASH_1_STATE_0: [u8; 16] = hex!("0d2cb592de56a89f47db82ccad3a98d7");
const HASH_1_STATE_1: [u8; 16] = hex!("6e998d3398b7c7155a129ef55780e7ac");
const HASH_1_STATE_2: [u8; 16] = hex!("1700776ad0c762ae6b507950e47ca0e8");
const HASH_1_STATE_3: [u8; 16] = hex!("0c240a638d82ad070500a1794849997e");
// xkey0, xkey1 = Hash256("RandomX AesHash1R xkeys")
const HASH_1_X_KEY_0: [u8; 16] = hex!("8983faf69f94248bbf56dc9001028906");
const HASH_1_X_KEY_1: [u8; 16] = hex!("d163b2613ce0f451c64310ee9bf918ed");

/// AesHash1R in the spec.
///
/// creates a 64 byte hash from the input.
///
/// https://github.com/tevador/RandomX/blob/master/doc/specs.md#34-aeshash1r
pub(crate) fn hash_aes_r1(buf: &[u8]) -> [u8; 64] {
    assert_eq!(buf.len() % 64, 0);

    let mut block_0 = Block::from(HASH_1_STATE_0);
    let mut block_1 = Block::from(HASH_1_STATE_1);
    let mut block_2 = Block::from(HASH_1_STATE_2);
    let mut block_3 = Block::from(HASH_1_STATE_3);

    for window in buf.windows(64) {
        aes_enc(&mut block_0, Block::from_slice(&window[0..16]));
        aes_dec(&mut block_1, Block::from_slice(&window[16..32]));
        aes_enc(&mut block_2, Block::from_slice(&window[32..48]));
        aes_dec(&mut block_3, Block::from_slice(&window[48..64]));
    }

    let x_key_0 = Block::from_slice(&HASH_1_X_KEY_0);
    aes_enc(&mut block_0, x_key_0);
    aes_dec(&mut block_1, x_key_0);
    aes_enc(&mut block_2, x_key_0);
    aes_dec(&mut block_3, x_key_0);

    let x_key_1 = Block::from_slice(&HASH_1_X_KEY_1);
    aes_enc(&mut block_0, x_key_1);
    aes_dec(&mut block_1, x_key_1);
    aes_enc(&mut block_2, x_key_1);
    aes_dec(&mut block_3, x_key_1);

    [block_0, block_1, block_2, block_3]
        .concat()
        .try_into()
        .unwrap()
}

/// AesGenerator1R in the spec.
///
/// Fills the bytes with pseudorandom bytes seeded by the input.
///
/// `output` must be a multiple of 64.
///
/// https://github.com/tevador/RandomX/blob/master/doc/specs.md#32-aesgenerator1r
pub(crate) fn aes_fill_1r(input: &[u8; 64], output: &mut [u8]) {
    assert_eq!(output.len() % 64, 0);

    let key_0 = Block::from(GENERATOR_1_KEY_0);
    let key_1 = Block::from(GENERATOR_1_KEY_1);
    let key_2 = Block::from(GENERATOR_1_KEY_2);
    let key_3 = Block::from(GENERATOR_1_KEY_3);

    let mut block_0 = Block::clone_from_slice(&input[0..16]);
    let mut block_1 = Block::clone_from_slice(&input[16..32]);
    let mut block_2 = Block::clone_from_slice(&input[32..48]);
    let mut block_3 = Block::clone_from_slice(&input[48..64]);

    for idx in (0..output.len()).step_by(64) {
        aes_dec(&mut block_0, &key_0);
        aes_enc(&mut block_1, &key_1);
        aes_dec(&mut block_2, &key_2);
        aes_enc(&mut block_3, &key_3);

        output[idx..idx + 16].clone_from_slice(block_0.as_slice());
        output[idx + 16..idx + 32].clone_from_slice(block_1.as_slice());
        output[idx + 32..idx + 48].clone_from_slice(block_2.as_slice());
        output[idx + 48..idx + 64].clone_from_slice(block_3.as_slice());
    }
}

/// AesGenerator4R in the spec.
///
/// Fills the output with pseudorandom bytes seeded by the input.
///
/// `output` must be a multiple of 64.
///
/// https://github.com/tevador/RandomX/blob/master/doc/specs.md#33-aesgenerator4r
pub(crate) fn aes_fill_4r(input: &[u8; 64], output: &mut [u8]) {
    assert_eq!(output.len() % 64, 0);

    let key_0 = Block::from(GENERATOR_4_KEY_0);
    let key_1 = Block::from(GENERATOR_4_KEY_1);
    let key_2 = Block::from(GENERATOR_4_KEY_2);
    let key_3 = Block::from(GENERATOR_4_KEY_3);
    let key_4 = Block::from(GENERATOR_4_KEY_4);
    let key_5 = Block::from(GENERATOR_4_KEY_5);
    let key_6 = Block::from(GENERATOR_4_KEY_6);
    let key_7 = Block::from(GENERATOR_4_KEY_7);

    let mut block_0 = Block::clone_from_slice(&input[0..16]);
    let mut block_1 = Block::clone_from_slice(&input[16..32]);
    let mut block_2 = Block::clone_from_slice(&input[32..48]);
    let mut block_3 = Block::clone_from_slice(&input[48..64]);

    let aes_enc_4 = |block: &mut Block, key_a, key_b, key_c, key_d| {
        aes_enc(block, key_a);
        aes_enc(block, key_b);
        aes_enc(block, key_c);
        aes_enc(block, key_d);
    };

    let aes_dec_4 = |block: &mut Block, key_a, key_b, key_c, key_d| {
        aes_dec(block, key_a);
        aes_dec(block, key_b);
        aes_dec(block, key_c);
        aes_dec(block, key_d);
    };

    for idx in (0..output.len()).step_by(64) {
        aes_dec_4(&mut block_0, &key_0, &key_1, &key_2, &key_3);
        aes_enc_4(&mut block_1, &key_0, &key_1, &key_2, &key_3);
        aes_dec_4(&mut block_2, &key_4, &key_5, &key_6, &key_7);
        aes_enc_4(&mut block_3, &key_4, &key_5, &key_6, &key_7);

        output[idx..idx + 16].clone_from_slice(block_0.as_slice());
        output[idx + 16..idx + 32].clone_from_slice(block_1.as_slice());
        output[idx + 32..idx + 48].clone_from_slice(block_2.as_slice());
        output[idx + 48..idx + 64].clone_from_slice(block_3.as_slice());
    }
}
