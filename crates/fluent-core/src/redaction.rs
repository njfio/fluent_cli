use once_cell::sync::Lazy;
use regex::Regex;

// Match key-value like: bearer_token: value, api_key=value, Authorization: secret
static RE_KV_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(bearer[_-]?token|api[_-]?key|authorization|x-api-key)\b\s*[:=]\s*[^\s]+"
    ).expect("valid regex")
});

// Match Authorization: Bearer TOKEN
static RE_AUTH_BEARER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bAuthorization\s*:\s*Bearer\s+[A-Za-z0-9._\-]+").expect("valid regex")
});

// Match URL query tokens: ?api_key=... or &token=...
static RE_URL_QUERY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([?&](?:api_key|token|key)=)[^&\s]+").expect("valid regex")
});

/// Redact common secret patterns from arbitrary text.
pub fn redact_secrets_in_text(input: &str) -> String {
    let step1 = RE_KV_SECRET.replace_all(input, |caps: &regex::Captures| {
        // Replace the whole match with '<key>: ***REDACTED***'
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("secret");
        format!("{}: ***REDACTED***", key)
    });
    let step2 = RE_AUTH_BEARER.replace_all(&step1, |_| {
        "Authorization: Bearer ***REDACTED***".to_string()
    });
    let step3 = RE_URL_QUERY.replace_all(&step2, |caps: &regex::Captures| {
        format!("{}REDACTED", &caps[1])
    });
    step3.into_owned()
}
