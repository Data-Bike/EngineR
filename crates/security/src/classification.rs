//! Classification levels and field/edge policy (docs/08 §7).
//!
//! Field can be allow, omit, null-mask, partial-mask, tokenize, or
//! aggregate-only. Omit is preferred: the key is absent. Masking is applied
//! in SQL/response projection and explicitly marked in meta only if allowed
//! to know about the field (docs/08 §7).

use serde::{Deserialize, Serialize};

/// Classification level (docs/08 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationLevel {
    /// Public: no restrictions.
    Public,
    /// Internal: available to authenticated users.
    Internal,
    /// Restricted: available to specific groups.
    Restricted,
    /// Confidential: available to specific individuals.
    Confidential,
    /// Secret: highest restriction.
    Secret,
}

impl ClassificationLevel {
    /// Get numeric rank (higher = more restricted).
    pub fn rank(self) -> u8 {
        match self {
            Self::Public => 0,
            Self::Internal => 1,
            Self::Restricted => 2,
            Self::Confidential => 3,
            Self::Secret => 4,
        }
    }

    /// Check if this level is allowed given a maximum allowed level.
    pub fn is_allowed_with(self, max_allowed: ClassificationLevel) -> bool {
        self.rank() <= max_allowed.rank()
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "public" => Some(Self::Public),
            "internal" => Some(Self::Internal),
            "restricted" => Some(Self::Restricted),
            "confidential" => Some(Self::Confidential),
            "secret" => Some(Self::Secret),
            _ => None,
        }
    }

    /// Convert to string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Restricted => "restricted",
            Self::Confidential => "confidential",
            Self::Secret => "secret",
        }
    }
}

/// Field policy kind (docs/08 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldPolicyKind {
    /// Field is allowed (full access).
    Allow,
    /// Field is omitted (key absent, preferred over mask, docs/08 §7).
    Omit,
    /// Field is null-masked (value replaced with null).
    NullMask,
    /// Field is partially masked (e.g. "1234****5678").
    PartialMask,
    /// Field is tokenized (replaced with a reversible token).
    Tokenize,
    /// Field is aggregate-only (only counts/sums, no individual values).
    AggregateOnly,
}

/// Mask strategy for field masking (docs/08 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskStrategy {
    /// Mask kind.
    pub kind: FieldPolicyKind,
    /// For partial mask: number of characters to show at start.
    pub show_prefix: Option<u32>,
    /// For partial mask: number of characters to show at end.
    pub show_suffix: Option<u32>,
    /// For partial mask: mask character.
    pub mask_char: Option<String>,
    /// For tokenize: tokenization key ID.
    pub tokenization_key_id: Option<String>,
}

impl MaskStrategy {
    /// Create a null-mask strategy.
    pub fn null_mask() -> Self {
        Self {
            kind: FieldPolicyKind::NullMask,
            show_prefix: None,
            show_suffix: None,
            mask_char: None,
            tokenization_key_id: None,
        }
    }

    /// Create a partial-mask strategy.
    pub fn partial_mask(show_prefix: u32, show_suffix: u32, mask_char: &str) -> Self {
        Self {
            kind: FieldPolicyKind::PartialMask,
            show_prefix: Some(show_prefix),
            show_suffix: Some(show_suffix),
            mask_char: Some(mask_char.to_string()),
            tokenization_key_id: None,
        }
    }

    /// Create a tokenize strategy.
    pub fn tokenize(key_id: &str) -> Self {
        Self {
            kind: FieldPolicyKind::Tokenize,
            show_prefix: None,
            show_suffix: None,
            mask_char: None,
            tokenization_key_id: Some(key_id.to_string()),
        }
    }

    /// Apply the mask to a value.
    pub fn apply(&self, value: &str) -> String {
        match self.kind {
            FieldPolicyKind::Allow => value.to_string(),
            FieldPolicyKind::Omit => String::new(),
            FieldPolicyKind::NullMask => "null".to_string(),
            FieldPolicyKind::PartialMask => {
                let prefix = self.show_prefix.unwrap_or(0) as usize;
                let suffix = self.show_suffix.unwrap_or(0) as usize;
                let mask_char = self.mask_char.as_deref().unwrap_or("*");
                let chars: Vec<char> = value.chars().collect();
                let len = chars.len();
                if len <= prefix + suffix {
                    return mask_char.repeat(len);
                }
                let mut result = String::new();
                for (i, c) in chars.iter().enumerate() {
                    if i < prefix || i >= len - suffix {
                        result.push(*c);
                    } else {
                        result.push_str(mask_char);
                    }
                }
                result
            }
            FieldPolicyKind::Tokenize => {
                // Tokenization would use a reversible tokenization algorithm
                // For now, return a placeholder token
                format!("tok_{}", value.len())
            }
            FieldPolicyKind::AggregateOnly => "aggregate_only".to_string(),
        }
    }
}

/// Field policy (docs/08 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldPolicy {
    /// Field name (predicate alias).
    pub field_name: String,
    /// Policy kind.
    pub kind: FieldPolicyKind,
    /// Mask strategy (if kind is NullMask/PartialMask/Tokenize).
    pub mask: Option<MaskStrategy>,
    /// Whether to mark the field as masked in meta (docs/08 §7).
    pub mark_in_meta: bool,
}

impl FieldPolicy {
    /// Create an allow policy.
    pub fn allow(field_name: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            kind: FieldPolicyKind::Allow,
            mask: None,
            mark_in_meta: false,
        }
    }

    /// Create an omit policy (preferred over mask, docs/08 §7).
    pub fn omit(field_name: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            kind: FieldPolicyKind::Omit,
            mask: None,
            mark_in_meta: false,
        }
    }

    /// Create a null-mask policy.
    pub fn null_mask(field_name: &str, mark_in_meta: bool) -> Self {
        Self {
            field_name: field_name.to_string(),
            kind: FieldPolicyKind::NullMask,
            mask: Some(MaskStrategy::null_mask()),
            mark_in_meta,
        }
    }

    /// Create a partial-mask policy.
    pub fn partial_mask(
        field_name: &str,
        show_prefix: u32,
        show_suffix: u32,
        mask_char: &str,
        mark_in_meta: bool,
    ) -> Self {
        Self {
            field_name: field_name.to_string(),
            kind: FieldPolicyKind::PartialMask,
            mask: Some(MaskStrategy::partial_mask(
                show_prefix,
                show_suffix,
                mask_char,
            )),
            mark_in_meta,
        }
    }

    /// Apply the policy to a value.
    pub fn apply(&self, value: &str) -> String {
        match &self.mask {
            Some(m) => m.apply(value),
            None => match self.kind {
                FieldPolicyKind::Allow => value.to_string(),
                FieldPolicyKind::Omit => String::new(),
                _ => value.to_string(),
            },
        }
    }
}
