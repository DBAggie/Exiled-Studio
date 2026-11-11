//! Share-code codec and errors on synthetic input; the public fixtures and
//! the PoB-made code are in `tests/share_code_fixtures.rs`.

use loom_domain::Game;

use super::*;

fn code_of(result: Result<impl Sized, ShareCodeError>) -> &'static str {
    result.err().map_or("ok", |e| e.code())
}

#[test]
fn base64_should_match_rfc_4648_with_the_url_safe_alphabet() {
    // RFC 4648, section 10.
    let vectors = [
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];
    for (plain, encoded) in vectors {
        assert_eq!(base64_encode(plain.as_bytes()), encoded);
        assert_eq!(base64_decode(encoded).unwrap(), plain.as_bytes());
    }
    assert_eq!(base64_encode(b""), "");
    // The two characters where the alphabets differ.
    assert_eq!(base64_encode(&[0xfb, 0xff]), "-_8=");
}

#[test]
fn base64_should_round_trip_every_byte_and_length() {
    let bytes: Vec<u8> = (0..=255).collect();
    for len in 0..bytes.len() {
        let encoded = base64_encode(&bytes[..len]);
        assert_eq!(encoded.len(), len.div_ceil(3) * 4);
        if len > 0 {
            assert_eq!(base64_decode(&encoded).unwrap(), &bytes[..len], "{len}");
        }
    }
}

#[test]
fn reading_should_accept_what_pob_accepts() {
    let want = vec![0xfb, 0xff];
    for text in ["-_8=", "+/8=", "-_8", " -_\r\n8= \t", "+_8"] {
        assert_eq!(base64_decode(text).unwrap(), want, "{text:?}");
    }
    // Spare bits after the last byte are not checked.
    assert_eq!(base64_decode("Zh==").unwrap(), b"f");
    assert_eq!(base64_decode("Zm9=").unwrap(), b"fo");
}

#[test]
fn a_code_over_its_limit_should_give_too_large_before_decoding() {
    let long = "A".repeat(MAX_CODE_BYTES + 1);
    assert_eq!(code_of(decode(&long)), "IMPORT_SHARE_CODE_TOO_LARGE");
    // At the limit it is decoded (and is no zlib stream).
    let at_limit = "A".repeat(MAX_CODE_BYTES);
    assert_eq!(code_of(decode(&at_limit)), "IMPORT_SHARE_CODE_ZLIB");
}

#[test]
fn bad_base64_should_give_its_code() {
    for text in [
        "", "  ", "Zm9v!", "Zm9vY", "Zm9v=Zg", "Zg===", "Zm8==", "Zg=", "é",
    ] {
        assert_eq!(
            code_of(base64_decode(text)),
            "IMPORT_SHARE_CODE_BASE64",
            "{text:?}"
        );
    }
}

#[test]
fn bytes_that_are_no_zlib_stream_should_give_its_code() {
    let whole = miniz_oxide::deflate::compress_to_vec_zlib(b"<PathOfBuilding/>", 9);
    for bytes in [
        b"not zlib".as_slice(),
        &whole[..whole.len() - 3],
        &whole[2..],
    ] {
        assert_eq!(
            code_of(decode(&base64_encode(bytes))),
            "IMPORT_SHARE_CODE_ZLIB"
        );
    }
}

#[test]
fn a_document_over_its_limit_should_give_too_large() {
    // An incompressible document of the maximum size still fits the
    // code limit.
    let mut state = 1_u32;
    let noise: Vec<u8> = (0..MAX_XML_BYTES)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            b'a' + (state % 26) as u8
        })
        .collect();
    let code = encode(std::str::from_utf8(&noise).unwrap());
    assert!(code.len() <= MAX_CODE_BYTES, "{}", code.len());
    assert_eq!(decode(&code).unwrap().len(), MAX_XML_BYTES);

    let big = vec![b' '; MAX_XML_BYTES + 1];
    let code = base64_encode(&miniz_oxide::deflate::compress_to_vec_zlib(&big, 9));
    // A few kilobytes of code for 32 MiB of text.
    assert!(code.len() < 100_000, "{}", code.len());
    assert_eq!(code_of(decode(&code)), "IMPORT_SHARE_CODE_TOO_LARGE");
    let at_limit = vec![b' '; MAX_XML_BYTES];
    let code = base64_encode(&miniz_oxide::deflate::compress_to_vec_zlib(&at_limit, 9));
    assert_eq!(decode(&code).unwrap().len(), MAX_XML_BYTES);
}

#[test]
fn a_document_not_in_utf8_should_give_its_code() {
    let code = base64_encode(&miniz_oxide::deflate::compress_to_vec_zlib(
        b"<PathOfBuilding \xe9/>",
        9,
    ));
    assert_eq!(code_of(decode(&code)), "IMPORT_SHARE_CODE_UTF8");
}

#[test]
fn a_document_that_does_not_import_should_give_the_xml_code() {
    assert_eq!(code_of(import(&encode("not xml"))), "IMPORT_XML");
    assert_eq!(code_of(import(&encode("<Other/>"))), "IMPORT_ROOT");
}

#[test]
fn bytes_after_the_stream_should_be_ignored() {
    let mut bytes = miniz_oxide::deflate::compress_to_vec_zlib(b"<PathOfBuilding/>", 9);
    bytes.extend_from_slice(b"tail");
    assert_eq!(decode(&base64_encode(&bytes)).unwrap(), "<PathOfBuilding/>");
}

#[test]
fn a_build_should_round_trip_through_its_code() {
    let build = Build::new(Game::Poe1);
    let code = export(&build);
    assert!(
        code.bytes().all(|c| ALPHABET.contains(&c) || c == b'='),
        "{code}"
    );
    assert_eq!(decode(&code).unwrap(), pob_xml::export(&build));
    assert_eq!(
        import(&code).unwrap(),
        pob_xml::import(&pob_xml::export(&build)).unwrap()
    );
    assert_eq!(export(&build), code, "a second run differs");
}
