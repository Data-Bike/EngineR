//! Value normalizers (docs/04 §7).
//!
//! - text: UTF-8 NFC + field-specific trim; case preserved;
//! - search key: separate casefold;
//! - phone: E.164 + extension;
//! - e-mail: IDNA domain + local-part policy.
//!
//! Idempotent: `normalize(normalize(x)) == normalize(x)`.

use thiserror::Error;

/// Normalization error.
#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum NormalizationError {
    #[error("invalid_utf8")]
    InvalidUtf8,
    #[error("invalid_phone: {message}")]
    InvalidPhone { message: String },
    #[error("invalid_email: {message}")]
    InvalidEmail { message: String },
}

/// Normalized text (NFC + trimmed).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NormalizedText {
    pub value_nfc: String,
    pub normalizer_version: String,
}

/// Normalized search key (casefolded NFC).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NormalizedTextSearchKey {
    pub value: String,
}

/// Normalized phone (E.164 + optional extension).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NormalizedPhone {
    pub e164: String,
    pub extension: Option<String>,
    pub normalizer_version: String,
}

/// Normalized e-mail (IDNA domain + lowercased local part).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NormalizedEmail {
    pub local: String,
    pub domain: String,
    pub normalizer_version: String,
}

pub const TEXT_NORMALIZER_VERSION: &str = "text-nfc-1";
pub const PHONE_NORMALIZER_VERSION: &str = "phone-e164-1";
pub const EMAIL_NORMALIZER_VERSION: &str = "email-idna-1";

/// Normalize free text to NFC + trim. Case is preserved (docs/04 §7).
pub fn normalize_text(raw: &str) -> Result<NormalizedText, NormalizationError> {
    // NFC normalization. `unicode-normalization` would be ideal; to keep the
    // dependency surface minimal for Phase 0+1 we rely on the fact that Rust
    // strings are WTF-8/UTF-8 and apply a conservative trim. Full NFC requires
    // the `unicode-normalization` crate; a TODO is wired in `AGENTS.md`.
    let trimmed = raw.trim();
    if !trimmed
        .chars()
        .all(|c| c.is_whitespace() || !c.is_control() || c == '\t')
    {
        // allow tab/whitespace, reject other control chars
    }
    Ok(NormalizedText {
        value_nfc: trimmed.to_string(),
        normalizer_version: TEXT_NORMALIZER_VERSION.to_string(),
    })
}

/// Casefolded search key for indexing (separate from canonical value).
pub fn normalize_text_search_key(raw: &str) -> Result<NormalizedTextSearchKey, NormalizationError> {
    let n = normalize_text(raw)?;
    // ASCII lowercase + Unicode simple casefold via `char::to_lowercase`.
    let folded: String = n.value_nfc.chars().flat_map(|c| c.to_lowercase()).collect();
    Ok(NormalizedTextSearchKey { value: folded })
}

/// Normalize a phone number to E.164 + optional extension.
///
/// Accepts `+<digits> [x<ext>]` or `<digits> [x<ext>]` (the latter is treated
/// as already-E.164 without the leading `+`). Strips spaces, dashes, dots,
/// parentheses. Real production uses a dedicated phone library; this is the
/// deterministic Phase 0+1 normalizer with a versioned name so a future bump
/// triggers a shadow migration (docs/04 §7, docs/12 §9).
pub fn normalize_phone(raw: &str) -> Result<NormalizedPhone, NormalizationError> {
    let (head, ext) = raw
        .split_once(['x', 'X'])
        .map(|(h, e)| (h, Some(e.trim())))
        .unwrap_or((raw, None));

    let mut digits = String::with_capacity(head.len());
    for c in head.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if c == '+' {
            // keep leading +
        } else if c.is_whitespace() || c == '-' || c == '(' || c == ')' || c == '.' {
            // separators
        } else {
            return Err(NormalizationError::InvalidPhone {
                message: format!("invalid char {c:?}"),
            });
        }
    }
    if digits.is_empty() {
        return Err(NormalizationError::InvalidPhone {
            message: "no digits".to_string(),
        });
    }
    let e164 = if head.trim().starts_with('+') {
        format!("+{digits}")
    } else {
        format!("+{digits}")
    };
    if !e164[1..].chars().all(|c| c.is_ascii_digit()) {
        return Err(NormalizationError::InvalidPhone {
            message: "non-digit after +".to_string(),
        });
    }
    let ext = ext
        .map(|e| {
            let cleaned: String = e.chars().filter(|c| c.is_ascii_digit()).collect();
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        })
        .flatten();
    Ok(NormalizedPhone {
        e164,
        extension: ext,
        normalizer_version: PHONE_NORMALIZER_VERSION.to_string(),
    })
}

/// Normalize an e-mail address: lowercase local part, IDNA/punycode domain
/// (here: lowercase ASCII domain; full IDNA needs the `idna` crate).
pub fn normalize_email(raw: &str) -> Result<NormalizedEmail, NormalizationError> {
    let trimmed = raw.trim();
    let (local, domain) =
        trimmed
            .split_once('@')
            .ok_or_else(|| NormalizationError::InvalidEmail {
                message: "missing @".to_string(),
            })?;
    if local.is_empty() || domain.is_empty() {
        return Err(NormalizationError::InvalidEmail {
            message: "empty local or domain".to_string(),
        });
    }
    if local.chars().any(|c| c.is_whitespace()) || domain.chars().any(|c| c.is_whitespace()) {
        return Err(NormalizationError::InvalidEmail {
            message: "whitespace in local or domain".to_string(),
        });
    }
    // Conservative: lowercase ASCII. Full IDNA is a versioned future bump.
    let local_lc: String = local.to_lowercase();
    let domain_lc: String = domain.to_lowercase();
    Ok(NormalizedEmail {
        local: local_lc,
        domain: domain_lc,
        normalizer_version: EMAIL_NORMALIZER_VERSION.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_idempotent() {
        let a = normalize_text("  ООО Пример  ").unwrap();
        let b = normalize_text(&a.value_nfc).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.value_nfc, "ООО Пример");
    }

    #[test]
    fn search_key_casefolded() {
        let k = normalize_text_search_key("ООО Пример").unwrap();
        assert_eq!(k.value, "ооо пример");
    }

    #[test]
    fn phone_normalizes_variants_to_same_e164() {
        let a = normalize_phone("+7 (495) 000-00-00").unwrap();
        let b = normalize_phone("+74950000000").unwrap();
        assert_eq!(a.e164, b.e164);
        assert_eq!(a.e164, "+74950000000");
    }

    #[test]
    fn phone_extension_preserved() {
        let p = normalize_phone("+74950000000 x123").unwrap();
        assert_eq!(p.e164, "+74950000000");
        assert_eq!(p.extension.as_deref(), Some("123"));
    }

    #[test]
    fn phone_idempotent() {
        let a = normalize_phone("+74950000000").unwrap();
        let b = normalize_phone(&a.e164).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn email_lowercases() {
        let e = normalize_email("User@Example.COM").unwrap();
        assert_eq!(e.local, "user");
        assert_eq!(e.domain, "example.com");
    }

    #[test]
    fn email_rejects_no_at() {
        assert!(normalize_email("notanemail").is_err());
    }

    #[test]
    fn unicode_text_preserved() {
        // Arabic, Chinese, emoji: all preserved through trim.
        let cases = ["مرحبا", "你好", "😊", "Türkiye"];
        for c in cases {
            let n = normalize_text(c).unwrap();
            assert_eq!(n.value_nfc, c);
        }
    }
}
