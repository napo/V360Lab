//! Parsing and validation of user-supplied camera addresses.
//!
//! The application only ever talks to the host the user configured. URLs
//! reported by the camera (media, thumbnails, FIT files) are re-anchored onto
//! that host before being requested.

use url::Url;

use super::CameraError;

/// Address of a VIRB camera acting as its own Wi-Fi access point.
/// Only a suggestion: on other networks the camera gets a different IP.
pub const DEFAULT_CAMERA_ADDRESS: &str = "192.168.0.1";

/// Normalizes `192.168.0.1`, `192.168.0.1:8080` or `http://virb.local` into a
/// base URL such as `http://192.168.0.1/`.
pub fn normalize_address(input: &str) -> Result<Url, CameraError> {
    let trimmed = input.trim();
    let invalid = |reason: &str| CameraError::InvalidAddress {
        address: trimmed.to_string(),
        reason: reason.to_string(),
    };

    if trimmed.is_empty() {
        return Err(invalid("the address is empty"));
    }
    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    let url = Url::parse(&candidate).map_err(|e| invalid(&e.to_string()))?;

    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(invalid("only http:// addresses are supported"));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(invalid("the host name is missing"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("credentials are not allowed in the address"));
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(invalid(
            "enter only a host name or IP address, optionally with a port",
        ));
    }
    Ok(url)
}

/// Resolves a URL reported by the camera against the configured base URL.
///
/// Relative URLs are joined to `base`. Absolute URLs keep their path and
/// query but are re-anchored onto the configured scheme, host and port, so
/// requests never leave the device the user selected (the camera reports its
/// own IP even when the user connected through a host name or port forward).
pub fn resolve_camera_url(base: &Url, reported: &str) -> Result<Url, CameraError> {
    let invalid = |reason: &str| CameraError::InvalidUrl {
        url: reported.to_string(),
        reason: reason.to_string(),
    };
    let reported = reported.trim();
    if reported.is_empty() {
        return Err(invalid("empty URL"));
    }
    let mut url = base.join(reported).map_err(|e| invalid(&e.to_string()))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(invalid("unsupported URL scheme"));
    }
    if url.scheme() != base.scheme() {
        url.set_scheme(base.scheme())
            .map_err(|_| invalid("cannot change URL scheme"))?;
    }
    url.set_host(base.host_str())
        .map_err(|e| invalid(&e.to_string()))?;
    url.set_port(base.port())
        .map_err(|_| invalid("cannot set URL port"))?;
    // Never forward credentials embedded in a reported URL.
    let _ = url.set_username("");
    let _ = url.set_password(None);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_plain_ip() {
        let url = normalize_address(" 192.168.0.1 ").unwrap();
        assert_eq!(url.as_str(), "http://192.168.0.1/");
    }

    #[test]
    fn keeps_port_and_scheme() {
        let url = normalize_address("http://virb.local:8080").unwrap();
        assert_eq!(url.as_str(), "http://virb.local:8080/");
    }

    #[test]
    fn rejects_invalid_addresses() {
        for bad in [
            "",
            "   ",
            "ftp://1.2.3.4",
            "1.2.3.4/virb",
            "http://u:p@1.2.3.4",
            "http://",
        ] {
            assert!(
                matches!(
                    normalize_address(bad),
                    Err(CameraError::InvalidAddress { .. })
                ),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn resolves_relative_and_foreign_urls_onto_configured_host() {
        let base = normalize_address("virb.local").unwrap();
        let with_port = normalize_address("127.0.0.1:8080").unwrap();
        assert_eq!(
            resolve_camera_url(&with_port, "http://192.168.0.1/DCIM/a.MP4")
                .unwrap()
                .as_str(),
            "http://127.0.0.1:8080/DCIM/a.MP4"
        );
        assert_eq!(
            resolve_camera_url(&base, "/DCIM/100_VIRB/V0010001.MP4")
                .unwrap()
                .as_str(),
            "http://virb.local/DCIM/100_VIRB/V0010001.MP4"
        );
        assert_eq!(
            resolve_camera_url(&base, "http://192.168.0.1/GMetrix/0001.fit")
                .unwrap()
                .as_str(),
            "http://virb.local/GMetrix/0001.fit"
        );
        assert_eq!(
            resolve_camera_url(&base, "https://evil.example.com:81/x?a=1")
                .unwrap()
                .as_str(),
            "http://virb.local/x?a=1"
        );
    }

    #[test]
    fn rejects_non_http_urls() {
        let base = normalize_address("192.168.0.1").unwrap();
        assert!(resolve_camera_url(&base, "file:///etc/passwd").is_err());
        assert!(resolve_camera_url(&base, "").is_err());
    }
}
