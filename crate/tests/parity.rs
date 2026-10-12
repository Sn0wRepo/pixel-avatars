//! Byte-for-byte parity with the browser generator (`web/index.html`).
//! `test/fixtures/avatars.json` was produced by the web page; the same file feeds the Foundry tests.

use pixelid::{blockies, parse_address, pixel_id};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../test/fixtures/avatars.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(&path).expect(&path)).unwrap()
}

#[test]
fn pixel_id_matches_web_and_solidity() {
    let fx = fixture();
    for item in fx["items"].as_array().unwrap() {
        let addr = item["address"].as_str().unwrap();
        let icon = pixel_id(&parse_address(addr).unwrap());
        assert_eq!(icon.to_svg(), item["pixelid"].as_str().unwrap(), "{addr}");
    }
}

#[test]
fn blockies_match_web_for_addresses() {
    let fx = fixture();
    for item in fx["items"].as_array().unwrap() {
        let addr = item["address"].as_str().unwrap();
        assert_eq!(
            blockies(addr, 8).to_svg(),
            item["blockies8"].as_str().unwrap(),
            "{addr} 8x8"
        );
        assert_eq!(
            blockies(addr, 12).to_svg(),
            item["blockies12"].as_str().unwrap(),
            "{addr} 12x12"
        );
    }
}

#[test]
fn blockies_match_web_for_text_and_unicode_seeds() {
    let fx = fixture();
    for item in fx["seeds"].as_array().unwrap() {
        let seed = item["seed"].as_str().unwrap();
        assert_eq!(
            blockies(seed, 8).to_svg(),
            item["blockies8"].as_str().unwrap(),
            "{seed:?} 8x8"
        );
        assert_eq!(
            blockies(seed, 5).to_svg(),
            item["blockies5"].as_str().unwrap(),
            "{seed:?} 5x5"
        );
    }
}

#[cfg(feature = "png")]
#[test]
fn png_has_expected_dimensions() {
    let png = pixel_id(&[7u8; 20]).to_png(16).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    // IHDR width/height (big-endian) at bytes 16..24.
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 128);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 128);
}
