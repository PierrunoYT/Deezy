use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes128;
use blowfish::Blowfish;
use cbc::Decryptor;
use cipher::block_padding::NoPadding;
use cipher::{BlockDecryptMut, KeyIvInit};

const XOR_MASK: u8 = 0xAB;

// "g4el58wc0zvf9na1" XOR 0xAB
const BF_SECRET: [u8; 16] = [
    0xCC, 0x9F, 0xCE, 0xC7, 0x9E, 0x93, 0xDC, 0xC8,
    0x9B, 0xD1, 0xDD, 0xCD, 0x92, 0xC5, 0xCA, 0x9A,
];

// "jo6aey6haid2Teih" XOR 0xAB
const AES_KEY: [u8; 16] = [
    0xC1, 0xC4, 0x9D, 0xCA, 0xCE, 0xD2, 0x9D, 0xC3,
    0xCA, 0xC2, 0xCF, 0x99, 0xFF, 0xCE, 0xC2, 0xC3,
];

fn deobfuscate(data: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = data[i] ^ XOR_MASK;
    }
    out
}

pub fn get_blowfish_key(track_id: &str) -> Result<Vec<u8>, String> {
    // An empty ID would still give a key (from md5("")), and with it silently
    // corrupted audio.
    if track_id.is_empty() {
        return Err("Track unavailable (missing SNG_ID)".to_string());
    }
    let secret = deobfuscate(&BF_SECRET);
    let id_bytes: Vec<u8> = track_id.chars().map(|c| c as u8).collect();
    let hash = md5::compute(&id_bytes);
    let hex = format!("{:x}", hash);
    let hex_bytes = hex.as_bytes();

    Ok((0..16)
        .map(|i| hex_bytes[i] ^ hex_bytes[i + 16] ^ secret[i])
        .collect())
}

pub fn encrypt_download_url(
    md5_origin: &str,
    quality_code: u32,
    track_id: &str,
    media_version: &str,
) -> Result<String, String> {
    let sep = '\u{00A4}';

    let step1 = format!(
        "{}{}{}{}{}{}{}",
        md5_origin, sep, quality_code, sep, track_id, sep, media_version
    );

    let step1_bytes: Vec<u8> = step1.chars().map(|c| c as u8).collect();
    let step1_md5 = format!("{:x}", md5::compute(&step1_bytes));

    let step2 = format!("{}{}{}{}", step1_md5, sep, step1, sep);
    // Pad to the next multiple of 16 (AES block size) with spaces.
    // Using a fixed width of 80 is insufficient for track IDs with 10+ digits.
    let step2_len = step2.chars().count();
    let padded_len = if step2_len % 16 == 0 {
        step2_len
    } else {
        step2_len + (16 - step2_len % 16)
    };
    let step2_padded = format!("{:<width$}", step2, width = padded_len);
    let step2_bytes: Vec<u8> = step2_padded.chars().map(|c| c as u8).collect();

    let aes_key = deobfuscate(&AES_KEY);
    let key = GenericArray::from_slice(&aes_key);
    let aes = Aes128::new(key);

    let mut result = String::new();
    for chunk in step2_bytes.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        aes.encrypt_block(&mut block);
        for b in block.iter() {
            result.push_str(&format!("{:02x}", b));
        }
    }

    // The first character picks the CDN host, so anything but a hex digit
    // (such as '@' or '/') would change where the request goes.
    let cdn = md5_origin
        .chars()
        .next()
        .filter(char::is_ascii_hexdigit)
        .ok_or("Track unavailable (invalid MD5_ORIGIN)")?;

    Ok(format!(
        "https://e-cdns-proxy-{}.dzcdn.net/mobile/1/{}",
        cdn, result
    ))
}

type BfCbcDec = Decryptor<Blowfish>;

pub fn decrypt_blowfish_chunk(chunk: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let iv = [0u8, 1, 2, 3, 4, 5, 6, 7];
    let mut buf = chunk.to_vec();
    let decrypted = BfCbcDec::new_from_slices(key, &iv)
        .map_err(|_| "Invalid key/iv length".to_string())?
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|_| "Decryption failed".to_string())?;
    Ok(decrypted.to_vec())
}

#[cfg(test)]
mod tests {
    use super::{encrypt_download_url, get_blowfish_key};

    #[test]
    fn an_empty_track_id_has_no_blowfish_key() {
        assert!(get_blowfish_key("").is_err());
        assert_eq!(get_blowfish_key("3135556").unwrap().len(), 16);
    }

    #[test]
    fn the_cdn_host_needs_a_hex_digit() {
        let url = encrypt_download_url("a1b2c3", 1, "3135556", "4").unwrap();
        assert!(url.starts_with("https://e-cdns-proxy-a.dzcdn.net/mobile/1/"), "{url}");
        assert!(encrypt_download_url("F00", 1, "3135556", "4").is_ok());

        for origin in ["@evil", "/x", "?q", "#f", "\\x", "g12", "é1", ""] {
            assert!(encrypt_download_url(origin, 1, "3135556", "4").is_err(), "{origin}");
        }
    }
}
