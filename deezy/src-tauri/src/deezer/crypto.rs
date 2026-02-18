use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes128;
use blowfish::Blowfish;
use cbc::Decryptor;
use cipher::block_padding::NoPadding;
use cipher::{BlockDecryptMut, KeyIvInit};

pub fn get_blowfish_key(track_id: &str) -> Vec<u8> {
    let secret = b"g4el58wc0zvf9na1";
    let id_bytes: Vec<u8> = track_id.chars().map(|c| c as u8).collect();
    let hash = md5::compute(&id_bytes);
    let hex = format!("{:x}", hash);
    let hex_bytes = hex.as_bytes();

    (0..16)
        .map(|i| hex_bytes[i] ^ hex_bytes[i + 16] ^ secret[i])
        .collect()
}

pub fn encrypt_download_url(
    md5_origin: &str,
    quality_code: u32,
    track_id: &str,
    media_version: &str,
) -> String {
    let sep = '\u{00A4}';

    let step1 = format!(
        "{}{}{}{}{}{}{}",
        md5_origin, sep, quality_code, sep, track_id, sep, media_version
    );

    let step1_bytes: Vec<u8> = step1.chars().map(|c| c as u8).collect();
    let step1_md5 = format!("{:x}", md5::compute(&step1_bytes));

    let step2 = format!("{}{}{}{}", step1_md5, sep, step1, sep);
    let step2_padded = format!("{:<80}", step2);
    let step2_bytes: Vec<u8> = step2_padded.chars().map(|c| c as u8).collect();

    let key = GenericArray::from_slice(b"jo6aey6haid2Teih");
    let aes = Aes128::new(key);

    let mut result = String::new();
    for chunk in step2_bytes.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        aes.encrypt_block(&mut block);
        for b in block.iter() {
            result.push_str(&format!("{:02x}", b));
        }
    }

    let cdn = &md5_origin[..1];
    format!(
        "https://e-cdns-proxy-{}.dzcdn.net/mobile/1/{}",
        cdn, result
    )
}

type BfCbcDec = Decryptor<Blowfish>;

pub fn decrypt_blowfish_chunk(chunk: &[u8], key: &[u8]) -> Vec<u8> {
    let iv = [0u8, 1, 2, 3, 4, 5, 6, 7];
    let mut buf = chunk.to_vec();
    let _ = BfCbcDec::new_from_slices(key, &iv)
        .expect("Invalid key/iv length")
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .expect("Decryption failed");
    buf
}
