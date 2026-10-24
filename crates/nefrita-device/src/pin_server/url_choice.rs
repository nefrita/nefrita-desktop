use ureq::http::Uri;

use crate::pin_server::PinServerError;

const HTTP_SCHEME: &str = "http";
const HTTPS_SCHEME: &str = "https";
const ONION_DOMAIN_SUFFIX: &str = ".onion";

pub(super) fn choose_url(offered_urls: &[String]) -> Result<&str, PinServerError> {
    let mut urls_reachable_without_tor = Vec::new();
    for offered_url in offered_urls {
        let invalid_url_error = || PinServerError::InvalidUrl(offered_url.clone());
        let parsed_url: Uri = offered_url.parse().map_err(|_| invalid_url_error())?;
        let uses_tls = match parsed_url.scheme_str() {
            Some(HTTPS_SCHEME) => true,
            Some(HTTP_SCHEME) => false,
            _ => return Err(invalid_url_error()),
        };
        let Some(host_name) = parsed_url.host() else {
            return Err(invalid_url_error());
        };
        if !is_onion_host(host_name) {
            urls_reachable_without_tor.push((offered_url.as_str(), uses_tls));
        }
    }
    urls_reachable_without_tor
        .iter()
        .find(|(_, uses_tls)| *uses_tls)
        .or_else(|| urls_reachable_without_tor.first())
        .map(|(reachable_url, _)| *reachable_url)
        .ok_or_else(|| PinServerError::NoUsableUrl(offered_urls.to_vec()))
}

fn is_onion_host(host_name: &str) -> bool {
    host_name
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .ends_with(ONION_DOMAIN_SUFFIX)
}

#[cfg(test)]
mod tests;
