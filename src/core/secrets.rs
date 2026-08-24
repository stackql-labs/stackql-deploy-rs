// core/secrets.rs

//! # Secrets Module
//!
//! Process-wide registry of sensitive values that must never appear in log
//! output. Values are registered when a manifest global or resource property
//! marked `protected: true` is rendered (and when resource-level `protected`
//! exports are captured), then every log line is scrubbed of registered
//! values at the logger sink (see `utils::logging`).
//!
//! Redaction is display-only: the real values are still stored in the
//! template context and sent to the server in queries.

use std::sync::RwLock;

use log::warn;

/// Fixed-width mask used in place of secret values. A fixed width is used so
/// that redacted output does not leak the length of the secret.
pub const MASK: &str = "********";

/// Values shorter than this are not registered: masking very short strings
/// (e.g. "1", "gp3") would garble unrelated log output.
const MIN_SECRET_LEN: usize = 4;

/// Registered secret values, kept sorted longest-first so that overlapping
/// secrets are replaced correctly.
static SECRETS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// Register a sensitive value for log redaction.
///
/// Also registers the JSON-string-escaped form of the value when it differs
/// (secrets can appear inside serialized JSON structures, e.g. tags).
pub fn register_secret(value: &str) {
    if value.is_empty() {
        return;
    }
    if value.len() < MIN_SECRET_LEN {
        warn!(
            "protected value is too short to mask reliably ({} chars), it will not be redacted from logs",
            value.len()
        );
        return;
    }

    let mut candidates = vec![value.to_string()];

    // JSON-escaped form (without the surrounding quotes), e.g. a secret
    // containing a double quote or backslash appears escaped inside JSON.
    if let Ok(escaped) = serde_json::to_string(value) {
        let inner = escaped.trim_matches('"');
        if inner != value {
            candidates.push(inner.to_string());
        }
    }

    let mut secrets = SECRETS.write().unwrap();
    for candidate in candidates {
        if candidate.len() < MIN_SECRET_LEN || secrets.contains(&candidate) {
            continue;
        }
        // Insert maintaining longest-first order
        let pos = secrets
            .iter()
            .position(|s| s.len() < candidate.len())
            .unwrap_or(secrets.len());
        secrets.insert(pos, candidate);
    }
}

/// Replace every occurrence of a registered secret value in `text` with the
/// fixed mask. Returns the input unchanged when no secrets are registered.
pub fn redact(text: &str) -> String {
    let secrets = SECRETS.read().unwrap();
    if secrets.is_empty() {
        return text.to_string();
    }
    let mut out = text.to_string();
    for secret in secrets.iter() {
        if out.contains(secret.as_str()) {
            out = out.replace(secret.as_str(), MASK);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: the registry is process-global and tests run in parallel, so each
    // test uses unique secret values and never clears the registry.

    #[test]
    fn test_redact_masks_registered_value() {
        register_secret("Sup3r-S3cret-Passw0rd");
        let out = redact("MasterUserPassword = 'Sup3r-S3cret-Passw0rd',");
        assert_eq!(out, format!("MasterUserPassword = '{}',", MASK));
    }

    #[test]
    fn test_redact_masks_multiple_occurrences() {
        register_secret("mult1-Occurrence-secret");
        let out = redact("a mult1-Occurrence-secret b mult1-Occurrence-secret c");
        assert_eq!(out, format!("a {} b {} c", MASK, MASK));
    }

    #[test]
    fn test_redact_leaves_other_text_alone() {
        register_secret("unrelated-secret-xyz123");
        let out = redact("no secrets here");
        assert_eq!(out, "no secrets here");
    }

    #[test]
    fn test_short_values_not_registered() {
        register_secret("ab1");
        let out = redact("value is ab1");
        assert_eq!(out, "value is ab1");
    }

    #[test]
    fn test_empty_value_not_registered() {
        register_secret("");
        let out = redact("some text");
        assert_eq!(out, "some text");
    }

    #[test]
    fn test_overlapping_secrets_longest_first() {
        register_secret("overlap-secret");
        register_secret("overlap-secret-longer-form");
        let out = redact("x overlap-secret-longer-form y");
        // The longer secret must be replaced whole, not partially by the shorter one
        assert_eq!(out, format!("x {} y", MASK));
    }

    #[test]
    fn test_json_escaped_form_registered() {
        register_secret(r#"pa"ss\word-with-specials"#);
        // As it would appear inside a serialized JSON string
        let json = serde_json::to_string(&serde_json::json!({
            "password": r#"pa"ss\word-with-specials"#
        }))
        .unwrap();
        let out = redact(&json);
        assert!(
            !out.contains("word-with-specials"),
            "escaped secret leaked: {}",
            out
        );
    }

    #[test]
    fn test_duplicate_registration_is_idempotent() {
        register_secret("dup-registration-secret");
        register_secret("dup-registration-secret");
        let out = redact("dup-registration-secret");
        assert_eq!(out, MASK);
    }
}
