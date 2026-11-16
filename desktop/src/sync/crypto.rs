//! Encryption before upload: AES-256-GCM, with keys from the passphrase by PBKDF2-HMAC-SHA256.
//! Both are in the Android platform too, so the phone app reads the same files.
//!
//! Deriving the keys takes about a fifth of a second, by design. A sync asks for them through
//! `derive_cached`, which derives them once while the passphrase and the remote folder's salt
//! stay the same.

use std::sync::{Mutex, PoisonError};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

use super::sign::{hex, hmac};

pub const ITERATIONS: u32 = 600_000;
const MAGIC: &[u8] = b"QUIRE1";

#[derive(Clone)]
pub struct Keys {
    enc: [u8; 32],
    names: [u8; 32],
}

/// 64 bytes from the passphrase: the first 32 encrypt, the last 32 name the objects.
pub fn derive(pass: &str, salt: &[u8], iterations: u32) -> Keys {
    #[cfg(test)]
    DERIVED.with(|n| n.set(n.get() + 1));
    let mut out = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(pass.as_bytes(), salt, iterations, &mut out);
    let mut enc = [0u8; 32];
    let mut names = [0u8; 32];
    enc.copy_from_slice(&out[..32]);
    names.copy_from_slice(&out[32..]);
    Keys { enc, names }
}

#[cfg(test)]
thread_local! {
    /// How many times this thread derived keys.
    static DERIVED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The keys derived last, with a hash of the passphrase, salt and iteration count they came from.
struct Last(Option<([u8; 32], Keys)>);

impl Last {
    fn keys(&mut self, pass: &str, salt: &[u8], iterations: u32) -> Keys {
        use sha2::Digest;
        // The salt's length goes in first, so two different inputs never hash the same bytes.
        let mut h = sha2::Sha256::new();
        h.update(iterations.to_be_bytes());
        h.update((salt.len() as u64).to_be_bytes());
        h.update(salt);
        h.update(pass.as_bytes());
        let from: [u8; 32] = h.finalize().into();
        if let Some((kept, keys)) = &self.0
            && *kept == from
        {
            return keys.clone();
        }
        let keys = derive(pass, salt, iterations);
        self.0 = Some((from, keys.clone()));
        keys
    }
}

/// `derive`, but the keys come from memory when the last call had the same passphrase, salt and
/// iteration count. The app syncs every few seconds while notes change, and each sync needs the
/// keys. Another passphrase or salt derives new keys and replaces the kept ones. Nothing is
/// written to disk.
pub fn derive_cached(pass: &str, salt: &[u8], iterations: u32) -> Keys {
    static LAST: Mutex<Last> = Mutex::new(Last(None));
    LAST.lock().unwrap_or_else(PoisonError::into_inner).keys(pass, salt, iterations)
}

pub fn random(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).expect("the system has a random source");
    b
}

/// `QUIRE1`, a 12-byte nonce, then the ciphertext with its tag.
pub fn seal(k: &Keys, plain: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new_from_slice(&k.enc).expect("32-byte key");
    let nonce = random(12);
    let ct = cipher.encrypt(Nonce::from_slice(&nonce), plain).expect("AES-GCM encrypts any length");
    let mut out = Vec::with_capacity(MAGIC.len() + 12 + ct.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    out
}

pub fn open(k: &Keys, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < MAGIC.len() + 12 + 16 || &data[..MAGIC.len()] != MAGIC {
        return Err("A remote file is not in Typo's encrypted format.".into());
    }
    let cipher = Aes256Gcm::new_from_slice(&k.enc).expect("32-byte key");
    let (nonce, ct) = data[MAGIC.len()..].split_at(12);
    cipher.decrypt(Nonce::from_slice(nonce), ct).map_err(|_| "The passphrase does not open the remote notes.".to_string())
}

/// The object name of a note, so the store never sees the note's path.
pub fn object_name(k: &Keys, path: &str) -> String {
    format!("objects/{}.bin", &hex(&hmac(&k.names, path.as_bytes()))[..40])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seals_and_opens() {
        let k = derive("correct horse battery", b"salt-salt-salt-s", 1000);
        let c = seal(&k, b"# Note");
        assert_eq!(open(&k, &c).unwrap(), b"# Note");
        let wrong = derive("another pass phrase", b"salt-salt-salt-s", 1000);
        assert!(open(&wrong, &c).is_err());
        assert_eq!(object_name(&k, "A.md"), object_name(&k, "A.md"));
        assert_ne!(object_name(&k, "A.md"), object_name(&k, "B.md"));
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn the_same_passphrase_and_salt_derive_once() {
        let derived = || DERIVED.with(std::cell::Cell::get);
        let both = |k: &Keys| (k.enc, k.names);
        let (pass, salt) = ("correct horse battery", &b"salt-salt-salt-s"[..]);
        let mut last = Last(None);
        let start = derived();
        let first = last.keys(pass, salt, 1000);
        let again = last.keys(pass, salt, 1000);
        assert_eq!(derived() - start, 1, "the second call derives nothing");
        assert_eq!(both(&first), both(&again));
        assert_eq!(both(&first), both(&derive(pass, salt, 1000)), "the kept keys are the derived ones");
        // Another passphrase, salt or iteration count never gets the kept keys.
        for (p, s, n) in [("another pass phrase", salt, 1000), (pass, &b"another-salt-16b"[..], 1000), (pass, salt, 1001)] {
            let k = last.keys(p, s, n);
            assert_eq!(both(&k), both(&derive(p, s, n)));
            assert_ne!(both(&k), both(&first));
        }
        // One set of keys is kept: the first passphrase derives again after another took its place.
        let before = derived();
        last.keys(pass, salt, 1000);
        assert_eq!(derived() - before, 1);
    }

    #[test]
    fn the_salt_and_the_passphrase_do_not_run_together() {
        // The same bytes split another way are another salt and passphrase.
        let mut last = Last(None);
        let a = last.keys("bc", b"a", 1000);
        let b = last.keys("c", b"ab", 1000);
        assert_ne!((a.enc, a.names), (b.enc, b.names));
        assert_eq!((b.enc, b.names), { let k = derive("c", b"ab", 1000); (k.enc, k.names) });
    }
}

#[cfg(test)]
mod vectors {
    use super::*;
    use base64::Engine;

    /// Prints values for other implementations: `cargo test print_vectors -- --nocapture`.
    #[test]
    fn print_vectors() {
        let salt = b"0123456789abcdef";
        let k = derive("correct horse battery staple", salt, 1000);
        let b = base64::engine::general_purpose::STANDARD;
        println!("salt_b64={}", b.encode(salt));
        println!("enc_key_hex={}", hex(&k.enc));
        println!("name_key_hex={}", hex(&k.names));
        println!("object_name(Drafts/Plan.md)={}", object_name(&k, "Drafts/Plan.md"));
        println!("sealed(# Blåbær)_b64={}", b.encode(seal(&k, "# Blåbær".as_bytes())));
        println!("check_b64={}", b.encode(seal(&k, b"quire")));
        println!("sha256(# Plan)={}", super::super::sign::sha256_hex(b"# Plan"));
    }
}
