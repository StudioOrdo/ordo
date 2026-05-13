use secrecy::{ExposeSecret, SecretString};

pub type OrdoSecretString = SecretString;

pub fn secret_string(value: impl Into<String>) -> OrdoSecretString {
    SecretString::from(value.into())
}

pub fn normalize_secret(value: impl Into<String>) -> Option<OrdoSecretString> {
    let value = value.into();
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| secret_string(trimmed.to_string()))
}

pub fn expose_secret(secret: &OrdoSecretString) -> &str {
    secret.expose_secret()
}

pub fn constant_time_secret_eq(candidate: &str, expected: &OrdoSecretString) -> bool {
    constant_time_eq::constant_time_eq(candidate.as_bytes(), expected.expose_secret().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_redacts_value() {
        let secret = secret_string("sk-test-secret-value");
        let debug = format!("{secret:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("sk-test-secret-value"));
    }

    #[test]
    fn normalized_secret_trims_and_rejects_empty_values() {
        let secret = normalize_secret("  secret-token  ").unwrap();

        assert_eq!(expose_secret(&secret), "secret-token");
        assert!(normalize_secret("  ").is_none());
    }

    #[test]
    fn constant_time_secret_comparison_checks_value() {
        let secret = secret_string("secret-token");

        assert!(constant_time_secret_eq("secret-token", &secret));
        assert!(!constant_time_secret_eq("secret-tokem", &secret));
        assert!(!constant_time_secret_eq("secret-token-extra", &secret));
    }
}
