use super::*;

fn offered_urls(urls: &[&str]) -> Vec<String> {
    urls.iter()
        .map(|offered_url| (*offered_url).to_owned())
        .collect()
}

#[test]
fn https_url_is_preferred() {
    let urls = offered_urls(&["http://plain.test/get_pin", "https://secure.test/get_pin"]);
    assert_eq!(choose_url(&urls), Ok("https://secure.test/get_pin"));
}

#[test]
fn http_url_is_used_when_no_https_url_is_offered() {
    let urls = offered_urls(&["http://localhost:8096/get_pin"]);
    assert_eq!(choose_url(&urls), Ok("http://localhost:8096/get_pin"));
}

#[test]
fn onion_urls_are_skipped_without_tor() {
    let urls = offered_urls(&[
        "https://PinServer.ONION/get_pin",
        "http://localhost:8096/get_pin",
    ]);
    assert_eq!(choose_url(&urls), Ok("http://localhost:8096/get_pin"));
}

#[test]
fn only_onion_urls_are_reported_as_unusable() {
    let urls = offered_urls(&["http://pinserver.onion/get_pin"]);
    assert_eq!(
        choose_url(&urls),
        Err(PinServerError::NoUsableUrl(urls.clone()))
    );
}

#[test]
fn missing_urls_are_reported_as_unusable() {
    assert_eq!(
        choose_url(&[]),
        Err(PinServerError::NoUsableUrl(Vec::new()))
    );
}

#[test]
fn malformed_url_is_reported_as_invalid() {
    let urls = offered_urls(&["http://pin server.test/get_pin"]);
    assert_eq!(
        choose_url(&urls),
        Err(PinServerError::InvalidUrl(urls[0].clone()))
    );
}

#[test]
fn url_with_another_scheme_is_reported_as_invalid() {
    let urls = offered_urls(&["ftp://pin.test/get_pin"]);
    assert_eq!(
        choose_url(&urls),
        Err(PinServerError::InvalidUrl(urls[0].clone()))
    );
}
