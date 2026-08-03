//! Excel import pipeline DTOs (docs/15 §3-6).
//!
//! Import is a job: upload → structure inspection → mapping profile selection →
//! preview + validation → dry run → approval → chunk execution → reconciliation.
//!
//! This module defines the mapping profile and row-level types. The actual
//! XLSX parsing is done by a separate crate (to avoid adding heavy deps to
//! contracts); this module only defines the data contracts.

use serde::{Deserialize, Serialize};

/// Error during import mapping.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ImportError {
    #[error("parse: {message}")]
    Parse { message: String },
    #[error("mapping: {message}")]
    Mapping { message: String },
    #[error("validation: {message}")]
    Validation { message: String },
    #[error("identity: {message}")]
    Identity { message: String },
    #[error("conflict: {message}")]
    Conflict { message: String },
}

/// A mapping profile defines how source columns map to target predicates
/// (docs/15 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingProfile {
    /// Profile ID.
    pub profile_id: String,
    /// Profile version.
    pub version: String,
    /// Source sheet name.
    pub sheet: String,
    /// Header row number (1-based).
    pub header_row: u32,
    /// Data start row (1-based, usually header_row + 1).
    pub data_start_row: u32,
    /// Column mappings.
    pub columns: Vec<ColumnMapping>,
    /// Identity policy ID to use for dedup.
    pub identity_policy_id: String,
    /// What to do on conflict.
    pub conflict_action: ConflictAction,
    /// How to handle empty cells.
    pub empty_semantics: EmptySemantics,
    /// Row filter expression (optional, DSL TBD).
    pub row_filter: Option<String>,
}

/// How a single source column maps to a target predicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnMapping {
    /// Source column header name.
    pub source_column: String,
    /// Target predicate ID.
    pub target_predicate_id: String,
    /// Target type ID (for entity-creation mappings).
    pub target_type_id: Option<String>,
    /// Parser kind for the cell value.
    pub parser: CellParser,
    /// Normalizer version to apply.
    pub normalizer: Option<String>,
    /// For entity-creation mappings: the relation predicate to create.
    pub relation: Option<RelationMapping>,
}

/// How to create a relation when mapping creates a new entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationMapping {
    /// Relation predicate ID (edge from source entity to created entity).
    pub relation_predicate_id: String,
    /// Direction: "outgoing" or "incoming".
    pub direction: String,
}

/// Cell value parser kind (docs/15 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellParser {
    /// Plain text (NFC normalized).
    Text,
    /// Date (ISO format).
    Date,
    /// Money (decimal + currency).
    Money,
    /// Phone (E.164 normalization).
    Phone,
    /// Email.
    Email,
    /// Integer.
    Integer,
    /// Decimal.
    Decimal,
    /// Boolean.
    Boolean,
    /// INN (Russian tax ID).
    Inn,
    /// OGRN (Russian registration number).
    Ogrn,
    /// KPP (Russian tax code).
    Kpp,
    /// OKVED (Russian activity code).
    Okved,
    /// OPF (Russian legal form).
    Opf,
    /// Person name (split into last/first/middle).
    PersonName,
}

/// Conflict action when a value already exists (docs/15 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAction {
    /// Create a new source-specific assertion (default, docs/15 §5).
    CreateNewAssertion,
    /// Skip the cell, keep existing value.
    Skip,
    /// Mark as conflict for review.
    MarkConflict,
    /// Overwrite (requires approval).
    Overwrite,
}

/// How to handle empty cells (docs/15 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmptySemantics {
    /// Skip the cell, do not modify existing value.
    Skip,
    /// Treat as null/empty assertion.
    Null,
    /// Use default value from mapping.
    Default,
}

/// A parsed row from the source file, ready for mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedRow {
    /// 1-based row number in the source file.
    pub row_number: u32,
    /// Cell values keyed by column header.
    pub cells: Vec<ParsedCell>,
    /// Row idempotency key = file hash + mapping version + sheet + row.
    pub idempotency_key: String,
}

/// A single parsed cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCell {
    /// Column header name.
    pub column: String,
    /// Raw cell value as string.
    pub raw_value: Option<String>,
    /// Parsed/normalized value (if successful).
    pub parsed_value: Option<String>,
    /// Parse error (if any).
    pub error: Option<String>,
}

/// Dry run result for a single row (docs/15 §3 step 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunRowResult {
    pub row_number: u32,
    /// "create", "update", "dedupe", "conflict", "error", "skip"
    pub outcome: RowOutcome,
    /// Target entity public ID (if resolved).
    pub entity_public_id: Option<String>,
    /// Statements to create.
    pub statements: Vec<PlannedStatement>,
    /// Error message (if outcome is error).
    pub error: Option<String>,
}

/// Row-level outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowOutcome {
    /// New entity will be created.
    Create,
    /// Existing entity will be updated.
    Update,
    /// Duplicate detected, existing entity returned.
    Dedupe,
    /// Conflict detected, needs review.
    Conflict,
    /// Error, row skipped.
    Error,
    /// Skipped (empty or filtered).
    Skip,
}

/// A planned statement from mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedStatement {
    pub predicate_id: String,
    pub value: String,
    /// "text", "date", "money", etc.
    pub value_kind: String,
    /// Source provenance.
    pub source: String,
    /// Whether this is an identity statement.
    pub is_identity: bool,
}

/// Dry run summary (docs/15 §3 step 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunSummary {
    pub total_rows: u32,
    pub creates: u32,
    pub updates: u32,
    pub dedupes: u32,
    pub conflicts: u32,
    pub errors: u32,
    pub skips: u32,
}

impl DryRunSummary {
    /// Create an empty summary.
    pub fn new() -> Self {
        Self {
            total_rows: 0,
            creates: 0,
            updates: 0,
            dedupes: 0,
            conflicts: 0,
            errors: 0,
            skips: 0,
        }
    }

    /// Accumulate a row result.
    pub fn accumulate(&mut self, result: &DryRunRowResult) {
        self.total_rows += 1;
        match result.outcome {
            RowOutcome::Create => self.creates += 1,
            RowOutcome::Update => self.updates += 1,
            RowOutcome::Dedupe => self.dedupes += 1,
            RowOutcome::Conflict => self.conflicts += 1,
            RowOutcome::Error => self.errors += 1,
            RowOutcome::Skip => self.skips += 1,
        }
    }
}

impl Default for DryRunSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a cell value using the specified parser (docs/15 §4).
///
/// Returns the normalized string value or an error message.
pub fn parse_cell(raw: &str, parser: CellParser) -> Result<String, String> {
    match parser {
        CellParser::Text => Ok(raw.trim().to_string()),
        CellParser::Date => {
            // Accept ISO or DD.MM.YYYY
            let trimmed = raw.trim();
            if trimmed.contains('.') {
                // DD.MM.YYYY → YYYY-MM-DD
                let parts: Vec<&str> = trimmed.split('.').collect();
                if parts.len() == 3 {
                    Ok(format!("{}-{}-{}", parts[2], parts[1], parts[0]))
                } else {
                    Err(format!("invalid date: {}", raw))
                }
            } else {
                Ok(trimmed.to_string())
            }
        }
        CellParser::Money => {
            // Remove currency symbols and whitespace, replace comma with dot
            let cleaned: String = raw
                .trim()
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '-')
                .collect::<String>()
                .replace(',', ".");
            // Validate as decimal (handle empty and single dot)
            if cleaned.is_empty() || cleaned == "." || cleaned == "-" {
                Ok(cleaned)
            } else if cleaned.parse::<f64>().is_ok() {
                Ok(cleaned)
            } else {
                Err(format!("invalid money: {}", raw))
            }
        }
        CellParser::Phone => {
            // Normalize to E.164 (basic: keep digits only, prepend +7 for RU)
            let digits: String = raw.trim().chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                Ok(String::new())
            } else if digits.starts_with('8') && digits.len() == 11 {
                Ok(format!("+7{}", &digits[1..]))
            } else if digits.starts_with('7') && digits.len() == 11 {
                Ok(format!("+{}", digits))
            } else {
                Ok(format!("+{}", digits))
            }
        }
        CellParser::Email => {
            let trimmed = raw.trim().to_lowercase();
            if trimmed.contains('@') && trimmed.contains('.') {
                Ok(trimmed)
            } else if trimmed.is_empty() {
                Ok(trimmed)
            } else {
                Err(format!("invalid email: {}", raw))
            }
        }
        CellParser::Integer => {
            let trimmed = raw.trim();
            if trimmed.parse::<i64>().is_ok() || trimmed.is_empty() {
                Ok(trimmed.to_string())
            } else {
                Err(format!("invalid integer: {}", raw))
            }
        }
        CellParser::Decimal => {
            let cleaned = raw.trim().replace(',', ".");
            if cleaned.parse::<f64>().is_ok() || cleaned.is_empty() {
                Ok(cleaned)
            } else {
                Err(format!("invalid decimal: {}", raw))
            }
        }
        CellParser::Boolean => {
            let lower = raw.trim().to_lowercase();
            match lower.as_str() {
                "true" | "1" | "да" | "yes" => Ok("true".to_string()),
                "false" | "0" | "нет" | "no" => Ok("false".to_string()),
                "" => Ok(String::new()),
                _ => Err(format!("invalid boolean: {}", raw)),
            }
        }
        CellParser::Inn => {
            let digits: String = raw.trim().chars().filter(|c| c.is_ascii_digit()).collect();
            // Russian INN: 10 digits (ЮЛ) or 12 digits (ФЛ)
            if digits.len() == 10 || digits.len() == 12 {
                Ok(digits)
            } else if digits.is_empty() {
                Ok(digits)
            } else {
                Err(format!("invalid INN (expected 10 or 12 digits): {}", raw))
            }
        }
        CellParser::Ogrn => {
            let digits: String = raw.trim().chars().filter(|c| c.is_ascii_digit()).collect();
            // Russian OGRN: 13 digits (ЮЛ) or OGRNIP: 15 digits (ИП)
            if digits.len() == 13 || digits.len() == 15 {
                Ok(digits)
            } else if digits.is_empty() {
                Ok(digits)
            } else {
                Err(format!("invalid OGRN (expected 13 or 15 digits): {}", raw))
            }
        }
        CellParser::Kpp => {
            let digits: String = raw.trim().chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 9 {
                Ok(digits)
            } else if digits.is_empty() {
                Ok(digits)
            } else {
                Err(format!("invalid KPP (expected 9 digits): {}", raw))
            }
        }
        CellParser::Okved => Ok(raw.trim().to_string()),
        CellParser::Opf => Ok(raw.trim().to_string()),
        CellParser::PersonName => {
            // Split "Last First Middle" into parts
            let parts: Vec<&str> = raw.trim().split_whitespace().collect();
            if parts.is_empty() {
                Ok(String::new())
            } else {
                // Return as "last|first|middle" format
                let last = parts.first().copied().unwrap_or("");
                let first = parts.get(1).copied().unwrap_or("");
                let middle = parts.get(2).copied().unwrap_or("");
                Ok(format!("{}|{}|{}", last, first, middle))
            }
        }
    }
}
