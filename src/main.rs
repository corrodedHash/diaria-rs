use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Generate, KeyInit},
};
use x448::{EphemeralSecret as X448PrivateKey, PublicKey as X448PublicKey};

fn generate_keypair() -> (X448PrivateKey, X448PublicKey) {
    let private_key = X448PrivateKey::generate();
    let public_key = X448PublicKey::from(&private_key);
    (private_key, public_key)
}

fn derive_shared_secret(
    private_key: &X448PrivateKey,
    peer_public_key: &X448PublicKey,
) -> chacha20poly1305::Key {
    let ikm = private_key.diffie_hellman(peer_public_key);

    let hk = hkdf::Hkdf::<sha2::Sha256>::new(None, ikm.as_bytes());
    let mut okm = chacha20poly1305::Key::default();
    hk.expand(b"diaria entry shared secret", &mut okm)
        .expect("42 is a valid length for Sha256 to output");

    okm
}

fn encrypt(
    long_term_public: &X448PublicKey,
    plaintext: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (ephemeral_private, ephemeral_public) = generate_keypair();
    let shared_secret = derive_shared_secret(&ephemeral_private, long_term_public);

    let cipher = XChaCha20Poly1305::new(&shared_secret);
    let nonce = chacha20poly1305::XNonce::generate();
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())?;

    let mut result = Vec::with_capacity(56 + 24 + ciphertext.len());
    result.extend_from_slice(ephemeral_public.as_bytes());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

fn decrypt(
    long_term_private: &X448PrivateKey,
    ciphertext: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    if ciphertext.len() < 80 {
        return Err("Ciphertext too short".into());
    }

    let ephemeral_public =
        X448PublicKey::from_bytes(&ciphertext[0..56]).ok_or("Invalid public key")?;
    let nonce = XNonce::try_from(&ciphertext[56..80])?;
    let actual_ciphertext = &ciphertext[80..];

    let shared_secret = derive_shared_secret(long_term_private, &ephemeral_public);
    let cipher = XChaCha20Poly1305::new_from_slice(&shared_secret)?;

    let plaintext = cipher.decrypt(&nonce, actual_ciphertext)?;
    Ok(String::from_utf8(plaintext)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (long_term_private, long_term_public) = generate_keypair();

    let message = "Hello, this is a secret message!";
    let encrypted = encrypt(&long_term_public, message)?;
    let decrypted = decrypt(&long_term_private, &encrypted)?;
    assert_eq!(message, decrypted);

    println!("String encryption/decryption successful!");
    Ok(())
}
