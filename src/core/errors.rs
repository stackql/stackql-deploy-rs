//! Fatal error detection for StackQL query execution.
//!
//! Maintains a list of error patterns that indicate unrecoverable failures
//! (network issues, auth failures, etc.) vs normal operational errors
//! (404 not found) that the retry/statecheck logic can handle.

/// Error patterns that indicate a fatal, non-retryable failure.
///
/// These are checked against the error message string returned by the
/// StackQL engine. If any pattern matches, the operation is aborted
/// immediately rather than retried.
///
/// Two categories:
///
/// 1. **Network errors** - The request never reached the API. Any result
///    from a query in this state is untrustworthy (e.g., an exists check
///    returning empty could cause a duplicate resource to be created).
///
/// 2. **HTTP status errors** - The request reached the API but the response
///    indicates an unrecoverable problem (auth failure, forbidden, etc.).
///    404 is explicitly excluded as it's normal for exists checks.
const FATAL_ERROR_PATTERNS: &[&str] = &[
    // Network-layer errors (Go net/http)
    "dial tcp:",
    "Client.Timeout exceeded",
    "connection refused",
    "no such host",
    "request canceled while waiting for connection",
    "request canceled (Client.Timeout",
    "tls: handshake",
    "certificate",
    "network is unreachable",
    "connection reset by peer",
    "broken pipe",
    "EOF",
    // HTTP status codes that are never retryable
    "http response status code: 401",
    "http response status code: 403",
];

/// Patterns that indicate a non-fatal error, even if a fatal pattern
/// also matches. These take precedence over `FATAL_ERROR_PATTERNS`.
///
/// For example, a 404 is normal for exists checks on resources that
/// don't exist yet.
const NON_FATAL_OVERRIDES: &[&str] = &[
    "http response status code: 404",
    "ResourceNotFoundException",
    "was not found",
];

/// Check if an error message indicates a fatal, non-retryable failure.
///
/// Returns `Some(reason)` if the error is fatal, `None` if it's
/// a normal operational error that can be retried or handled.
pub fn check_fatal_error(error_msg: &str) -> Option<&'static str> {
    // First check if any non-fatal override matches
    for pattern in NON_FATAL_OVERRIDES {
        if error_msg.contains(pattern) {
            return None;
        }
    }

    // Then check for fatal patterns
    FATAL_ERROR_PATTERNS
        .iter()
        .find(|&&pattern| error_msg.contains(pattern))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_timeout_is_fatal() {
        let msg = r#"Query execution failed: query returns error: Post "https://cloudcontrolapi.us-east-1.amazonaws.com/?Action=GetResource&Version=2021-09-30": net/http: request canceled while waiting for connection (Client.Timeout exceeded while awaiting headers)"#;
        assert!(check_fatal_error(msg).is_some());
    }

    #[test]
    fn test_dns_failure_is_fatal() {
        let msg = r#"Query execution failed: query returns error: Post "https://cloudcontrolapi.us-east-1.amazonaws.com/": dial tcp: lookup cloudcontrolapi.us-east-1.amazonaws.com on 8.8.4.4:53: i/o timeout"#;
        assert!(check_fatal_error(msg).is_some());
    }

    #[test]
    fn test_403_is_fatal() {
        let msg = r#"http response status code: 403, response body: {"message":"Access Denied"}"#;
        assert!(check_fatal_error(msg).is_some());
    }

    #[test]
    fn test_401_is_fatal() {
        let msg = r#"http response status code: 401, response body: {"message":"Unauthorized"}"#;
        assert!(check_fatal_error(msg).is_some());
    }

    #[test]
    fn test_404_is_not_fatal() {
        let msg = r#"http response status code: 404, response body: {"__type":"ResourceNotFoundException","Message":"Resource not found"}"#;
        assert!(check_fatal_error(msg).is_none());
    }

    #[test]
    fn test_resource_not_found_is_not_fatal() {
        let msg = r#"Resource of type 'AWS::EC2::VPC' with identifier 'vpc-xxx' was not found"#;
        assert!(check_fatal_error(msg).is_none());
    }

    #[test]
    fn test_400_bad_request_is_not_fatal() {
        let msg = r#"insert over HTTP error: 400 Bad Request"#;
        assert!(check_fatal_error(msg).is_none());
    }

    #[test]
    fn test_normal_query_error_is_not_fatal() {
        let msg = r#"query returns error: no such column: foo"#;
        assert!(check_fatal_error(msg).is_none());
    }
}
