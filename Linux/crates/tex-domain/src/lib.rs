//! Portable domain primitives for the Pitex document model.
//! Port of `Packages/TexCore/Sources/TexDomain/TexDomain.swift`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TexDomainValidationError {
    EmptyIdentifier,
    IdentifierTooLong { maximum: usize },
    InvalidIdentifierCharacter,
    EmptyPath,
    AbsolutePath,
    PathEscapesRoot,
    InvalidPathCharacter,
    PathComponentTooLong { maximum: usize },
}

impl fmt::Display for TexDomainValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier => write!(f, "empty identifier"),
            Self::IdentifierTooLong { maximum } => write!(f, "identifier too long (max {maximum})"),
            Self::InvalidIdentifierCharacter => write!(f, "invalid identifier character"),
            Self::EmptyPath => write!(f, "empty path"),
            Self::AbsolutePath => write!(f, "absolute path"),
            Self::PathEscapesRoot => write!(f, "path escapes root"),
            Self::InvalidPathCharacter => write!(f, "invalid path character"),
            Self::PathComponentTooLong { maximum } => {
                write!(f, "path component too long (max {maximum})")
            }
        }
    }
}

impl std::error::Error for TexDomainValidationError {}

fn validate_stable_identifier(value: &str) -> Result<(), TexDomainValidationError> {
    if value.is_empty() {
        return Err(TexDomainValidationError::EmptyIdentifier);
    }
    if value.len() > 128 {
        return Err(TexDomainValidationError::IdentifierTooLong { maximum: 128 });
    }
    if !value.bytes().all(|b| {
        b.is_ascii_digit() || b.is_ascii_uppercase() || b.is_ascii_lowercase() || b == b'-' || b == b'_'
    }) {
        return Err(TexDomainValidationError::InvalidIdentifierCharacter);
    }
    Ok(())
}

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            raw_value: String,
        }

        impl $name {
            pub fn new(raw_value: impl Into<String>) -> Result<Self, TexDomainValidationError> {
                let raw_value = raw_value.into();
                validate_stable_identifier(&raw_value)?;
                Ok(Self { raw_value })
            }

            pub fn raw_value(&self) -> &str {
                &self.raw_value
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.raw_value)
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.raw_value.cmp(&other.raw_value)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.raw_value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

stable_id!(StableProjectID);
stable_id!(StableDocumentID);

/// A project-root-relative POSIX path with `.`/`..` normalization applied at
/// construction time. Traversal above the root is rejected, matching the
/// Swift `NormalizedRelativePath` semantics exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRelativePath {
    raw_value: String,
}

impl NormalizedRelativePath {
    pub fn new(raw_value: impl Into<String>) -> Result<Self, TexDomainValidationError> {
        let raw_value = raw_value.into();
        if raw_value.is_empty() {
            return Err(TexDomainValidationError::EmptyPath);
        }
        if raw_value.starts_with('/') || raw_value.starts_with('\\') {
            return Err(TexDomainValidationError::AbsolutePath);
        }

        let original_components: Vec<&str> = raw_value.split('/').collect();
        if let Some(first) = original_components.first() {
            // Swift compares the second *Character*, not byte.
            let mut chars = first.chars();
            if chars.next().is_some() && chars.next() == Some(':') {
                return Err(TexDomainValidationError::AbsolutePath);
            }
        }

        let mut normalized_components: Vec<&str> = Vec::new();
        for component in original_components {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." {
                if normalized_components.is_empty() {
                    return Err(TexDomainValidationError::PathEscapesRoot);
                }
                normalized_components.pop();
                continue;
            }
            if component.len() > 255 {
                return Err(TexDomainValidationError::PathComponentTooLong { maximum: 255 });
            }
            if component.contains('\\') || component.contains('\0') {
                return Err(TexDomainValidationError::InvalidPathCharacter);
            }
            normalized_components.push(component);
        }

        if normalized_components.is_empty() {
            return Err(TexDomainValidationError::EmptyPath);
        }
        Ok(Self {
            raw_value: normalized_components.join("/"),
        })
    }

    pub fn raw_value(&self) -> &str {
        &self.raw_value
    }
}

impl fmt::Display for NormalizedRelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw_value)
    }
}

impl Hash for NormalizedRelativePath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw_value.hash(state)
    }
}

impl PartialOrd for NormalizedRelativePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NormalizedRelativePath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw_value.cmp(&other.raw_value)
    }
}

impl Serialize for NormalizedRelativePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw_value)
    }
}

impl<'de> Deserialize<'de> for NormalizedRelativePath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// SHA-256, ported from the reference implementation used across the app for
/// content fingerprints and SyncTeX output binding.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Base64url without padding (RFC 4648 §5).
pub fn base64_url_nopad(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

/// Lowercase hex rendering used for content-hash strings.
pub fn hex_lower(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

/// FNV-1a 64-bit — the `DiskContentHash`/`SnapshotRevision` fingerprint
/// function. Same constants as the Swift implementation.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}
