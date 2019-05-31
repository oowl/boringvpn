use ring::{aead,pbkdf2,digest,rand};
use std::num::NonZeroU32;
use ring::rand::SecureRandom;
use crate::types::Error;

const SALT: &[u8; 32] = b"junjunjunjunjunjunjunjunjunjunai";

pub enum CryptoMethod {
    ChaCha20,
    AES256
}

pub struct CryptoData {
    sealing_key: aead::SealingKey,
    opening_key: aead::OpeningKey,
    nonce: Vec<u8>,
    key: Vec<u8>
}

pub enum Crypto {
    None,
    ChaCha20Poly1305(CryptoData),
    AES256GCM(CryptoData)
}

fn inc_nonce(nonce: &mut [u8]) {
    let l = nonce.len();
    for i in (0..l).rev() {
        let mut num = nonce[i];
        num = num.wrapping_add(1);
        nonce[i] = num;
        if num > 0 {
            return
        }
    }
    log::warn!("Nonce overflowed");
}

impl Crypto {
    pub fn method(&self) -> u8 {
        match *self {
            Crypto::None => 0,
            Crypto::ChaCha20Poly1305 {..} => 1,
            Crypto::AES256GCM {..} => 2
        }
    }
    pub fn nonce_byte(&self) -> usize {
        match *self {
            Crypto::None => 0,
            Crypto::ChaCha20Poly1305(ref data) | Crypto::AES256GCM(ref data) => data.sealing_key.algorithm().nonce_len()
        }
    }
    pub fn get_key(&self) -> &[u8] {
        match *self {
            Crypto::None => &[],
            Crypto::ChaCha20Poly1305(ref data) | Crypto::AES256GCM(ref data) => &data.key
        }
    }
    pub fn additional_bytes(&self) -> usize {
        match *self {
            Crypto::None => 0,
            Crypto::ChaCha20Poly1305(ref data) | Crypto::AES256GCM(ref data) => data.sealing_key.algorithm().tag_len()
        }
    }
    pub fn from_shared_key(method: CryptoMethod,password: &str) -> Self {
        let algo = match method {
            CryptoMethod::ChaCha20 => &aead::CHACHA20_POLY1305,
            CryptoMethod::AES256 => &aead::AES_256_GCM
        };
        let mut key: Vec<u8> = Vec::with_capacity(algo.key_len());
        for _ in 0..algo.key_len() {
            key.push(0);
        }
        let sealing_key = aead::SealingKey::new(algo, &key[..algo.key_len()]).expect("Failed to create key");
        let opening_key = aead::OpeningKey::new(algo, &key[..algo.key_len()]).expect("Failed to create key");
        pbkdf2::derive(&digest::SHA256, NonZeroU32::new(4096).unwrap(), SALT, password.as_bytes(), &mut key);
        let mut nonce: Vec<u8> = Vec::with_capacity(algo.nonce_len());
        for _ in 0..algo.nonce_len() {
            nonce.push(0);
        }
        // leave the highest byte of the nonce 0 so it will not overflow
        if rand::SystemRandom::new().fill(&mut nonce[1..]).is_err() {
            log::warn!("Randomizing nonce failed");
        }
        let data = CryptoData { sealing_key, opening_key, nonce, key };
        match method {
            CryptoMethod::ChaCha20 => Crypto::ChaCha20Poly1305(data),
            CryptoMethod::AES256 => Crypto::AES256GCM(data)
        }
    }

    pub fn decrypt(&self, buf: &mut [u8], nonce: &[u8], header: &[u8]) -> Result<usize, Error> {
        match *self {
            Crypto::None => Ok(buf.len()),
            Crypto::ChaCha20Poly1305(ref data) | Crypto::AES256GCM(ref data) => {
                let nonce = aead::Nonce::try_assume_unique_for_key(nonce).unwrap();
                match aead::open_in_place(&data.opening_key, nonce, aead::Aad::from(header), 0, buf) {
                   Ok(plaintext) => Ok(plaintext.len()),
                   Err(_) => Err(Error::Crypto("Failed to decrypt"))
                }
            }
        }
    }
    pub fn encrypt(&mut self, buf: &mut [u8], mlen: usize, nonce_bytes: &mut [u8], header: &[u8]) -> usize {
        let tag_len = self.additional_bytes();
        match *self {
            Crypto::None => mlen,
            Crypto::ChaCha20Poly1305(ref mut data) | Crypto::AES256GCM(ref mut data) => {
                inc_nonce(&mut data.nonce);
                assert!(buf.len() - mlen >= tag_len);
                let buf = &mut buf[.. mlen + tag_len];
                let nonce = aead::Nonce::try_assume_unique_for_key(&data.nonce).unwrap();
                let new_len = aead::seal_in_place(&data.sealing_key, nonce, aead::Aad::from(header), buf, tag_len).expect("Failed to encrypt");
                nonce_bytes.clone_from_slice(&data.nonce);
                new_len
            }
        }
    }

}

#[test]
fn encrypt_decrypt_aes256() {
    let mut sender = Crypto::from_shared_key(CryptoMethod::AES256, "test");
    let receiver = Crypto::from_shared_key(CryptoMethod::AES256, "test");
    let msg = "HelloWorld0123456789";
    let msg_bytes = msg.as_bytes();
    let mut buffer = [0u8; 1024];
    let header = [0u8; 8];
    for i in 0..msg_bytes.len() {
        buffer[i] = msg_bytes[i];
    }
    let mut nonce1 = [0u8; 12];
    let size = sender.encrypt(&mut buffer, msg_bytes.len(), &mut nonce1, &header);
    assert_eq!(size, msg_bytes.len() + sender.additional_bytes());
    assert!(msg_bytes != &buffer[..msg_bytes.len()] as &[u8]);
    receiver.decrypt(&mut buffer[..size], &nonce1, &header).unwrap();
    assert_eq!(msg_bytes, &buffer[..msg_bytes.len()] as &[u8]);
    let mut nonce2 = [1u8; 12];
    let size = sender.encrypt(&mut buffer, msg_bytes.len(), &mut nonce2, &header);
    assert!(nonce1 != nonce2);
    receiver.decrypt(&mut buffer[..size], &nonce2, &header).unwrap();
    assert_eq!(msg_bytes, &buffer[..msg_bytes.len()] as &[u8]);
}
