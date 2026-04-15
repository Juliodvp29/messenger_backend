use sha2::{Digest, Sha256};

pub fn hash_phone(phone: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(phone.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}
