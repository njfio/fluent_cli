use once_cell::sync::Lazy;
use regex::Regex;

static RE_KV_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(bearer[_-]?token|api[_-]?key|authorization|x-api-key)\s*[:=]\s*([\"']?)([^\s\"']+)(\2)"
    ).expect("valid regex")
});

static RE_AUTH_BEARER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(Authorization)\s*:\s*(Bearer)\s+([A-Za-z0-9._\-]+)").expect("valid regex")
});

static RE_URL_QUERY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([?&](?:api_key|token|key)=)[^&\s]+").expect("valid regex")
});

/// Redact common secret patterns from arbitrary text.
pub fn redact_secrets_in_text(input: &str) -> String {
    let step1 = RE_KV_SECRET.replace_all(input, |caps: &regex::Captures| {
        format!("{}: ***REDACTED***", &caps[1])
    });
    let step2 = RE_AUTH_BEARER.replace_all(&step1, |caps: &regex::Captures| {
        format!("{}: {} ***REDACTED***", &caps[1], &caps[2])
    });
    let step3 = RE_URL_QUERY.replace_all(&step2, |caps: &regex::Captures| {
        format!("{}REDACTED", &caps[1])
    });
    step3.into_owned()
}
