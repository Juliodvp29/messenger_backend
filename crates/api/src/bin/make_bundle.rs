use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

fn main() {
    let mut csprng = OsRng;

    // Identity Key (Ed25519)
    let mut seed = [0u8; 32];
    rand::RngCore::fill_bytes(&mut csprng, &mut seed);
    let identity_signing_key = SigningKey::from_bytes(&seed);
    let identity_public_key = identity_signing_key.verifying_key();
    let identity_key_b64 = BASE64.encode(identity_public_key.as_bytes());

    // Signed Prekey (32 random bytes)
    let mut spk_bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut csprng, &mut spk_bytes);
    let signed_prekey_b64 = BASE64.encode(&spk_bytes);

    // Signature over the signed prekey bytes using identity key
    let signature = identity_signing_key.sign(&spk_bytes);
    let signature_b64 = BASE64.encode(signature.to_bytes());

    // One-Time Prekeys
    let mut opk1 = [0u8; 32];
    let mut opk2 = [0u8; 32];
    rand::RngCore::fill_bytes(&mut csprng, &mut opk1);
    rand::RngCore::fill_bytes(&mut csprng, &mut opk2);

    println!("=== Variables para Postman ===");
    println!("Reemplaza los valores en el Body de Postman con estos:\n");
    println!("\"identity_key\": \"{}\"", identity_key_b64);
    println!("\"key\": \"{}\"", signed_prekey_b64);
    println!("\"signature\": \"{}\"", signature_b64);
    println!("\"key\": \"{}\" (para OPK_1)", BASE64.encode(&opk1));
    println!("\"key\": \"{}\" (para OPK_2)", BASE64.encode(&opk2));
}
