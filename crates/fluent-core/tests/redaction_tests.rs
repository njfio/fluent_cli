use fluent_core::redaction::redact_secrets_in_text;

#[test]
fn test_redact_bearer_token_kv() {
    let input = "bearer_token: sk-abcdef123";
    let out = redact_secrets_in_text(input);
    assert!(out.contains("bearer_token: ***REDACTED***"));
    assert!(!out.contains("abcdef123"));
}

#[test]
fn test_redact_authorization_bearer_header() {
    let input = "Authorization: Bearer sk-abcdef123";
    let out = redact_secrets_in_text(input);
    assert_eq!(out, "Authorization: Bearer ***REDACTED***");
}

#[test]
fn test_redact_url_query_token() {
    let input = "https://api.example.com/path?api_key=sk-abcdef&other=1";
    let out = redact_secrets_in_text(input);
    assert!(out.contains("api_key=REDACTED"));
    assert!(!out.contains("sk-abcdef"));
}
