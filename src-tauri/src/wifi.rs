//! Validation of Wi-Fi networks before they are saved on the camera.
//!
//! The camera answers `"result": 0` to details it cannot use, without
//! saying why, so the rules of the Wi-Fi standards are checked here first
//! (Garmin's app shows the same warnings: SSID length, WPA password length
//! and characters, WEP key length).

use crate::camera::WifiSecurity;
use crate::error::AppError;

/// Longest SSID allowed by IEEE 802.11, in bytes.
const MAX_SSID_BYTES: usize = 32;

fn invalid(reason: &'static str, message: &'static str) -> AppError {
    AppError::Wifi { reason, message }
}

fn is_hex(text: &str) -> bool {
    text.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_printable_ascii(text: &str) -> bool {
    text.chars().all(|c| (' '..='~').contains(&c))
}

/// Checks a network before `configureNetwork`. Returns the password to
/// send: always empty for open networks.
pub fn validate<'a>(
    ssid: &str,
    security: WifiSecurity,
    password: &'a str,
) -> Result<&'a str, AppError> {
    if ssid.trim().is_empty() {
        return Err(invalid("ssidEmpty", "the network name is empty"));
    }
    if ssid.len() > MAX_SSID_BYTES {
        return Err(invalid(
            "ssidLength",
            "the network name is longer than 32 bytes",
        ));
    }
    match security {
        WifiSecurity::Open => Ok(""),
        WifiSecurity::Wpa | WifiSecurity::Wpa2 => {
            let len = password.chars().count();
            if len == 64 && is_hex(password) {
                Ok(password)
            } else if !(8..=63).contains(&len) {
                Err(invalid(
                    "wpaPasswordLength",
                    "WPA passwords have 8 to 63 characters",
                ))
            } else if !is_printable_ascii(password) {
                Err(invalid(
                    "wpaPasswordCharacters",
                    "WPA passwords may only contain ASCII characters",
                ))
            } else {
                Ok(password)
            }
        }
        WifiSecurity::Wep => {
            let valid = match password.len() {
                5 | 13 => is_printable_ascii(password),
                10 | 26 => is_hex(password),
                _ => false,
            };
            if valid {
                Ok(password)
            } else {
                Err(invalid(
                    "wepPasswordLength",
                    "WEP keys have 5 or 13 characters, or 10 or 26 hexadecimal digits",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(result: Result<&str, AppError>) -> &'static str {
        match result {
            Err(AppError::Wifi { reason, .. }) => reason,
            other => panic!("expected a Wi-Fi error, got {other:?}"),
        }
    }

    #[test]
    fn accepts_valid_networks() {
        assert_eq!(
            validate("Home", WifiSecurity::Wpa2, "password1").unwrap(),
            "password1"
        );
        assert_eq!(
            validate("Home", WifiSecurity::Wpa, &"a1".repeat(32))
                .unwrap()
                .len(),
            64
        );
        assert_eq!(
            validate("Old", WifiSecurity::Wep, "abcde").unwrap(),
            "abcde"
        );
        assert_eq!(
            validate("Old", WifiSecurity::Wep, "0123456789").unwrap(),
            "0123456789"
        );
        assert_eq!(validate("Cafe", WifiSecurity::Open, "ignored").unwrap(), "");
    }

    #[test]
    fn rejects_what_the_camera_cannot_use() {
        assert_eq!(reason(validate(" ", WifiSecurity::Open, "")), "ssidEmpty");
        assert_eq!(
            reason(validate(&"x".repeat(33), WifiSecurity::Open, "")),
            "ssidLength"
        );
        assert_eq!(
            reason(validate("Home", WifiSecurity::Wpa2, "short")),
            "wpaPasswordLength"
        );
        assert_eq!(
            reason(validate("Home", WifiSecurity::Wpa2, "pàssword1")),
            "wpaPasswordCharacters"
        );
        assert_eq!(
            reason(validate("Old", WifiSecurity::Wep, "abcdef")),
            "wepPasswordLength"
        );
        assert_eq!(
            reason(validate("Old", WifiSecurity::Wep, "zzzzzzzzzz")),
            "wepPasswordLength"
        );
    }
}
