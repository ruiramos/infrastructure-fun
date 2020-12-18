use magic_crypt::{MagicCryptError, MagicCryptTrait};
use rand::Rng;

pub fn encrypt_data(data: String) -> (String, String, String) {
    let key = generate_password(16);
    let mc = new_magic_crypt!(&key, 256);
    let encrypted = encrypt_to_base64(&data, mc);
    let hash = generate_hash(8);
    (hash, key, encrypted)
}

pub fn decrypt_data(data: String, key: String) -> Option<String> {
    let mc = new_magic_crypt!(&key, 256);
    mc.decrypt_base64_to_string(data).ok()
}

fn generate_password(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$#@!~";
    let mut rng = rand::thread_rng();

    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0, CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn generate_hash(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    let mut rng = rand::thread_rng();

    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0, CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn encrypt_to_base64(data: &str, mc: impl MagicCryptTrait) -> String {
    mc.encrypt_str_to_base64(data)
}
