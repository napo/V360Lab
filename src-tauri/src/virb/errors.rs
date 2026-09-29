//! Mapping of VIRB transport and protocol failures to [`CameraError`].

use std::error::Error as _;
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::Value;

use crate::camera::CameraError;

/// Maximum length of response excerpts kept in errors.
const ERROR_SNIPPET_CHARS: usize = 512;

/// Classifies a `reqwest` failure (timeout vs. unreachable vs. transfer).
pub fn from_reqwest(err: &reqwest::Error, address: &str, timeout: Duration) -> CameraError {
    let detail = error_chain(err);
    if err.is_timeout() {
        CameraError::Timeout {
            timeout_secs: timeout.as_secs(),
            detail,
        }
    } else if err.is_body() || err.is_decode() {
        CameraError::Transfer {
            url: err.url().map(|u| u.to_string()).unwrap_or_default(),
            detail,
        }
    } else {
        CameraError::Unreachable {
            address: address.to_string(),
            detail,
        }
    }
}

/// Rejects non-2xx responses. Firmware 4.20 answers an unknown command with
/// HTTP 400; 404/405/501 are how other HTTP servers answer unknown routes.
/// All of these are reported as "command not supported".
pub fn check_http_status(command: &str, status: StatusCode, body: &str) -> Result<(), CameraError> {
    if status.is_success() {
        return Ok(());
    }
    match status.as_u16() {
        400 | 404 | 405 | 501 => Err(CameraError::UnsupportedCommand {
            command: command.to_string(),
            detail: format!("HTTP {status}: {}", snippet(body, ERROR_SNIPPET_CHARS)),
        }),
        code => Err(CameraError::Http {
            status: code,
            body: snippet(body, ERROR_SNIPPET_CHARS),
        }),
    }
}

/// Parses a response body, which must be a JSON object.
pub fn parse_json(command: &str, body: &str) -> Result<Value, CameraError> {
    let value: Value = serde_json::from_str(body).map_err(|e| CameraError::MalformedResponse {
        command: command.to_string(),
        detail: format!("invalid JSON: {e}"),
        snippet: snippet(body, ERROR_SNIPPET_CHARS),
    })?;
    if !value.is_object() {
        return Err(CameraError::MalformedResponse {
            command: command.to_string(),
            detail: "expected a JSON object".to_string(),
            snippet: snippet(body, ERROR_SNIPPET_CHARS),
        });
    }
    Ok(value)
}

/// Checks the VIRB `result` field (`1` = success, `0` = failure).
/// A missing field is accepted, since not every response carries it.
pub fn check_result(command: &str, response: &Value) -> Result<(), CameraError> {
    let Some(result) = response.get("result") else {
        return Ok(());
    };
    match result_code(result) {
        Some(1) => Ok(()),
        Some(_) if looks_unsupported(response) => Err(CameraError::UnsupportedCommand {
            command: command.to_string(),
            detail: snippet(&response.to_string(), ERROR_SNIPPET_CHARS),
        }),
        Some(_) => Err(CameraError::CommandFailed {
            command: command.to_string(),
            response: snippet(&response.to_string(), ERROR_SNIPPET_CHARS),
        }),
        None => Err(CameraError::MalformedResponse {
            command: command.to_string(),
            detail: format!("unexpected \"result\" value: {result}"),
            snippet: snippet(&response.to_string(), ERROR_SNIPPET_CHARS),
        }),
    }
}

fn result_code(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

fn looks_unsupported(response: &Value) -> bool {
    ["error", "message", "reason"]
        .iter()
        .filter_map(|key| response.get(*key).and_then(Value::as_str))
        .map(str::to_lowercase)
        .any(|text| {
            [
                "unsupported",
                "not supported",
                "unknown command",
                "invalid command",
            ]
            .iter()
            .any(|needle| text.contains(needle))
        })
}

/// Truncates text on a character boundary, for logs and error details.
pub fn snippet(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((cut, _)) => format!("{}… ({} bytes total)", &text[..cut], text.len()),
        None => text.to_string(),
    }
}

fn error_chain(err: &reqwest::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut source = err.source();
    while let Some(inner) = source {
        parts.push(inner.to_string());
        source = inner.source();
    }
    parts.join(": ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_success_and_missing_result() {
        assert!(check_result("status", &json!({ "result": 1 })).is_ok());
        assert!(check_result("status", &json!({ "result": "1" })).is_ok());
        assert!(check_result("status", &json!({ "state": "idle" })).is_ok());
    }

    #[test]
    fn maps_result_zero_to_command_failed() {
        let err = check_result("startRecording", &json!({ "result": 0 })).unwrap_err();
        assert!(
            matches!(err, CameraError::CommandFailed { ref command, .. } if command == "startRecording")
        );
    }

    #[test]
    fn maps_unsupported_messages() {
        let err =
            check_result("foo", &json!({ "result": 0, "error": "Unknown command" })).unwrap_err();
        assert!(matches!(err, CameraError::UnsupportedCommand { .. }));
    }

    #[test]
    fn rejects_unexpected_result_type() {
        let err = check_result("status", &json!({ "result": [1] })).unwrap_err();
        assert!(matches!(err, CameraError::MalformedResponse { .. }));
    }

    #[test]
    fn classifies_http_errors() {
        assert!(matches!(
            check_http_status("x", StatusCode::NOT_FOUND, ""),
            Err(CameraError::UnsupportedCommand { .. })
        ));
        assert!(matches!(
            check_http_status("x", StatusCode::INTERNAL_SERVER_ERROR, "boom"),
            Err(CameraError::Http { status: 500, .. })
        ));
        assert!(check_http_status("x", StatusCode::OK, "").is_ok());
    }

    #[test]
    fn parse_json_rejects_garbage_and_non_objects() {
        assert!(matches!(
            parse_json("status", "<html>oops</html>"),
            Err(CameraError::MalformedResponse { .. })
        ));
        assert!(matches!(
            parse_json("status", "[1,2,3]"),
            Err(CameraError::MalformedResponse { .. })
        ));
        assert!(parse_json("status", r#"{"result":1}"#).is_ok());
    }

    #[test]
    fn snippet_truncates_on_char_boundary() {
        assert_eq!(snippet("abc", 10), "abc");
        assert!(snippet("ààààà", 2).starts_with("àà…"));
    }
}
