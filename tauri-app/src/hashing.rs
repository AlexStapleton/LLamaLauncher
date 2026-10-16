/// Stable lowercase encoding for persisted identifiers and published checksums.
/// Kept independent of the digest crate's output-array formatting traits.
pub fn hex(bytes: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 15) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::hex;
    use sha2::{Digest, Sha256};

    #[test]
    fn sha256_encoding_preserves_existing_identifiers_and_checksums() {
        assert_eq!(
            hex(Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(Sha256::digest(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(hex([0, 1, 15, 16, 255]), "00010f10ff");
    }
}
