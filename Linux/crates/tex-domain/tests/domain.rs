use tex_domain::*;

#[test]
fn stable_identifier_validation() {
    assert!(StableProjectID::new("multifile").is_ok());
    assert!(StableDocumentID::new("a-b_C9").is_ok());
    assert_eq!(
        StableDocumentID::new(""),
        Err(TexDomainValidationError::EmptyIdentifier)
    );
    assert_eq!(
        StableDocumentID::new("has space"),
        Err(TexDomainValidationError::InvalidIdentifierCharacter)
    );
    assert_eq!(
        StableDocumentID::new("has/slash"),
        Err(TexDomainValidationError::InvalidIdentifierCharacter)
    );
    assert_eq!(
        StableDocumentID::new("unicodé"),
        Err(TexDomainValidationError::InvalidIdentifierCharacter)
    );
    let too_long = "x".repeat(129);
    assert_eq!(
        StableDocumentID::new(too_long),
        Err(TexDomainValidationError::IdentifierTooLong { maximum: 128 })
    );
    let exact = "x".repeat(128);
    assert!(StableDocumentID::new(exact).is_ok());
}

#[test]
fn relative_path_normalization() {
    assert_eq!(
        NormalizedRelativePath::new("sections/./intro.tex")
            .unwrap()
            .raw_value(),
        "sections/intro.tex"
    );
    assert_eq!(
        NormalizedRelativePath::new("a/b/../c.tex").unwrap().raw_value(),
        "a/c.tex"
    );
    assert_eq!(
        NormalizedRelativePath::new("a//b.tex").unwrap().raw_value(),
        "a/b.tex"
    );
    assert_eq!(
        NormalizedRelativePath::new(""),
        Err(TexDomainValidationError::EmptyPath)
    );
    assert_eq!(
        NormalizedRelativePath::new("///"),
        Err(TexDomainValidationError::AbsolutePath)
    );
    assert_eq!(
        NormalizedRelativePath::new("/abs/path"),
        Err(TexDomainValidationError::AbsolutePath)
    );
    assert_eq!(
        NormalizedRelativePath::new("\\abs\\path"),
        Err(TexDomainValidationError::AbsolutePath)
    );
    assert_eq!(
        NormalizedRelativePath::new("C:\\dir\\file"),
        Err(TexDomainValidationError::AbsolutePath)
    );
    // Second *character* is ':' even for a multi-byte first character.
    assert_eq!(
        NormalizedRelativePath::new("é:x"),
        Err(TexDomainValidationError::AbsolutePath)
    );
    assert_eq!(
        NormalizedRelativePath::new("../escape"),
        Err(TexDomainValidationError::PathEscapesRoot)
    );
    assert_eq!(
        NormalizedRelativePath::new("a/../../escape"),
        Err(TexDomainValidationError::PathEscapesRoot)
    );
    assert_eq!(
        NormalizedRelativePath::new("back\\slash"),
        Err(TexDomainValidationError::InvalidPathCharacter)
    );
    let long_component = "x".repeat(256);
    assert_eq!(
        NormalizedRelativePath::new(long_component),
        Err(TexDomainValidationError::PathComponentTooLong { maximum: 255 })
    );
    assert_eq!(
        NormalizedRelativePath::new("./"),
        Err(TexDomainValidationError::EmptyPath)
    );
}

#[test]
fn ordering_matches_swift_comparable() {
    let a = StableDocumentID::new("a").unwrap();
    let b = StableDocumentID::new("b").unwrap();
    assert!(a < b);
    let p1 = NormalizedRelativePath::new("a/x.tex").unwrap();
    let p2 = NormalizedRelativePath::new("b.tex").unwrap();
    assert!(p1 < p2 || p2 < p1); // just Ord consistency
}

#[test]
fn serde_round_trip_is_a_json_string() {
    let id = StableDocumentID::new("main").unwrap();
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"main\"");
    let decoded: StableDocumentID = serde_json::from_str("\"main\"").unwrap();
    assert_eq!(decoded, id);
    assert!(serde_json::from_str::<StableDocumentID>("\"bad id\"").is_err());
}

#[test]
fn sha256_matches_reference_vectors() {
    assert_eq!(
        hex_lower(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex_lower(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let long = vec![b'a'; 1_000_000];
    assert_eq!(
        hex_lower(&sha256(&long)),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn base64url_nopad_vectors() {
    assert_eq!(base64_url_nopad(b""), "");
    assert_eq!(base64_url_nopad(b"f"), "Zg");
    assert_eq!(base64_url_nopad(b"fo"), "Zm8");
    assert_eq!(base64_url_nopad(b"foo"), "Zm9v");
    assert_eq!(base64_url_nopad(b"foob"), "Zm9vYg");
    assert_eq!(base64_url_nopad(&[0xfb, 0xff, 0xfe]), "-__-");
}

#[test]
fn fnv1a64_matches_swift_constants() {
    // FNV-1a 64: offset basis 14695981039346656037, prime 1099511628211.
    assert_eq!(fnv1a64(b""), 14_695_981_039_346_656_037);
    // Reference value computed from the same algorithm.
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for b in b"baseline" {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    assert_eq!(fnv1a64(b"baseline"), hash);
}
