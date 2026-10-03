use super::csrf::{check_origin, session_cookie};
use axum::http::HeaderMap;

#[test]
fn unsafe_requests_require_one_exact_origin_without_normalizing_attacker_input() {
    for invalid in [
        None,
        Some("https://studio.example.evil"),
        Some("https://studio.example/"),
        Some("null"),
        Some("http://studio.example"),
    ] {
        let mut headers = HeaderMap::new();
        if let Some(value) = invalid {
            headers.insert("origin", value.parse().unwrap());
        }
        assert!(check_origin(&headers, "https://studio.example").is_err());
    }
    let mut headers = HeaderMap::new();
    headers.insert("origin", "https://studio.example".parse().unwrap());
    assert!(check_origin(&headers, "https://studio.example").is_ok());
    headers.append("origin", "https://attacker.example".parse().unwrap());
    assert!(check_origin(&headers, "https://studio.example").is_err());
}

#[test]
fn duplicate_cookie_names_cannot_select_a_different_session() {
    let mut headers = HeaderMap::new();
    headers.insert("cookie", "other=abc; rom_session=good".parse().unwrap());
    assert_eq!(session_cookie(&headers, "rom_session").unwrap(), "good");
    headers.append("cookie", "rom_session=evil".parse().unwrap());
    assert!(session_cookie(&headers, "rom_session").is_err());
    headers.clear();
    headers.insert(
        "cookie",
        "rom_session=good; rom_session=evil".parse().unwrap(),
    );
    assert!(session_cookie(&headers, "rom_session").is_err());
}
