use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use super::*;

const TOR_ROUTE: PinServerRoute = PinServerRoute::Tor {
    socks_proxy_address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9050)),
};

fn offered_urls(urls: &[&str]) -> Vec<String> {
    urls.iter()
        .map(|offered_url| (*offered_url).to_owned())
        .collect()
}

#[test]
fn https_url_is_preferred() {
    let urls = offered_urls(&["http://plain.test/get_pin", "https://secure.test/get_pin"]);
    assert_eq!(
        choose_url(&urls, PinServerRoute::Direct),
        Ok("https://secure.test/get_pin")
    );
}

#[test]
fn http_url_is_used_when_no_https_url_is_offered() {
    let urls = offered_urls(&["http://localhost:8096/get_pin"]);
    assert_eq!(
        choose_url(&urls, PinServerRoute::Direct),
        Ok("http://localhost:8096/get_pin")
    );
}

#[test]
fn onion_urls_are_skipped_without_tor() {
    let urls = offered_urls(&[
        "https://PinServer.ONION/get_pin",
        "http://localhost:8096/get_pin",
    ]);
    assert_eq!(
        choose_url(&urls, PinServerRoute::Direct),
        Ok("http://localhost:8096/get_pin")
    );
}

#[test]
fn only_onion_urls_are_reported_as_unusable() {
    let urls = offered_urls(&["http://pinserver.onion/get_pin"]);
    assert_eq!(
        choose_url(&urls, PinServerRoute::Direct),
        Err(PinServerError::NoUsableUrl(urls.clone()))
    );
}

#[test]
fn missing_urls_are_reported_as_unusable() {
    assert_eq!(
        choose_url(&[], PinServerRoute::Direct),
        Err(PinServerError::NoUsableUrl(Vec::new()))
    );
}

#[test]
fn malformed_url_is_reported_as_invalid() {
    let urls = offered_urls(&["http://pin server.test/get_pin"]);
    assert_eq!(
        choose_url(&urls, PinServerRoute::Direct),
        Err(PinServerError::InvalidUrl(urls[0].clone()))
    );
}

#[test]
fn url_with_another_scheme_is_reported_as_invalid() {
    let urls = offered_urls(&["ftp://pin.test/get_pin"]);
    assert_eq!(
        choose_url(&urls, PinServerRoute::Direct),
        Err(PinServerError::InvalidUrl(urls[0].clone()))
    );
}

#[test]
fn onion_url_is_chosen_through_tor() {
    let urls = offered_urls(&[
        "https://secure.test/get_pin",
        "http://PinServer.ONION/get_pin",
    ]);
    assert_eq!(
        choose_url(&urls, TOR_ROUTE),
        Ok("http://PinServer.ONION/get_pin")
    );
}

#[test]
fn https_onion_url_is_preferred_through_tor() {
    let urls = offered_urls(&["http://plain.onion/get_pin", "https://secure.onion/get_pin"]);
    assert_eq!(
        choose_url(&urls, TOR_ROUTE),
        Ok("https://secure.onion/get_pin")
    );
}

#[test]
fn only_normal_urls_are_reported_as_unusable_through_tor() {
    let urls = offered_urls(&[
        "https://secure.test/get_pin",
        "http://localhost:8096/get_pin",
    ]);
    assert_eq!(
        choose_url(&urls, TOR_ROUTE),
        Err(PinServerError::NoOnionUrl(urls.clone()))
    );
}

#[test]
fn malformed_url_is_reported_as_invalid_through_tor() {
    let urls = offered_urls(&["http://pin server.onion/get_pin"]);
    assert_eq!(
        choose_url(&urls, TOR_ROUTE),
        Err(PinServerError::InvalidUrl(urls[0].clone()))
    );
}
