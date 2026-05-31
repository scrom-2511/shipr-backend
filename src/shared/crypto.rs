use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use std::env;

pub struct Crypto;

impl Crypto {
    fn get_key() -> Vec<u8> {
        let key_str = env::var("ENCRYPTION_KEY").unwrap_or_else(|_| "01234567890123456789012345678901".to_string());
        key_str.as_bytes()[..32].to_vec()
    }

    pub fn encrypt(data: &str) -> String {
        let key = Self::get_key();
        let cipher = Aes256Gcm::new_from_slice(&key).expect("Invalid key length");
        let nonce = Nonce::from_slice(b"unique nonce"); // In a real app, use a unique nonce per encryption and store it
        
        let ciphertext = cipher
            .encrypt(nonce, data.as_bytes())
            .expect("encryption failure!");
        
        general_purpose::STANDARD.encode(ciphertext)
    }

    pub fn decrypt(encrypted_data: &str) -> String {
        let key = Self::get_key();
        let cipher = Aes256Gcm::new_from_slice(&key).expect("Invalid key length");
        let nonce = Nonce::from_slice(b"unique nonce");
        
        let ciphertext = general_purpose::STANDARD
            .decode(encrypted_data)
            .expect("base64 decode failure");
            
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .expect("decryption failure!");
            
        String::from_utf8(plaintext).expect("Invalid UTF-8")
    }
}
