//! Excel import pipeline tests (docs/15 §3-6).

#![cfg(test)]

use enginer_contracts::import::{parse_cell, CellParser};

#[test]
fn parse_text_trims_whitespace() {
    assert_eq!(parse_cell("  hello  ", CellParser::Text).unwrap(), "hello");
}

#[test]
fn parse_date_dd_mm_yyyy_to_iso() {
    assert_eq!(
        parse_cell("01.02.2023", CellParser::Date).unwrap(),
        "2023-02-01"
    );
}

#[test]
fn parse_date_iso_passthrough() {
    assert_eq!(
        parse_cell("2023-02-01", CellParser::Date).unwrap(),
        "2023-02-01"
    );
}

#[test]
fn parse_money_removes_currency() {
    assert_eq!(parse_cell("10 000,50", CellParser::Money).unwrap(), "10000.50");
}

#[test]
fn parse_phone_normalizes_ru_8() {
    assert_eq!(
        parse_cell("8 (999) 123-45-67", CellParser::Phone).unwrap(),
        "+79991234567"
    );
}

#[test]
fn parse_phone_normalizes_ru_7() {
    assert_eq!(
        parse_cell("+7 (999) 123-45-67", CellParser::Phone).unwrap(),
        "+79991234567"
    );
}

#[test]
fn parse_email_lowercases() {
    assert_eq!(
        parse_cell("  John.Doe@Example.COM  ", CellParser::Email).unwrap(),
        "john.doe@example.com"
    );
}

#[test]
fn parse_email_rejects_invalid() {
    assert!(parse_cell("not-an-email", CellParser::Email).is_err());
}

#[test]
fn parse_inn_10_digits_legal() {
    assert_eq!(
        parse_cell("7707083893", CellParser::Inn).unwrap(),
        "7707083893"
    );
}

#[test]
fn parse_inn_12_digits_person() {
    assert_eq!(
        parse_cell("500100732259", CellParser::Inn).unwrap(),
        "500100732259"
    );
}

#[test]
fn parse_inn_rejects_wrong_length() {
    assert!(parse_cell("12345", CellParser::Inn).is_err());
}

#[test]
fn parse_ogrn_13_digits() {
    assert_eq!(
        parse_cell("1027700132195", CellParser::Ogrn).unwrap(),
        "1027700132195"
    );
}

#[test]
fn parse_ogrn_rejects_wrong_length() {
    assert!(parse_cell("12345", CellParser::Ogrn).is_err());
}

#[test]
fn parse_kpp_9_digits() {
    assert_eq!(parse_cell("773101001", CellParser::Kpp).unwrap(), "773101001");
}

#[test]
fn parse_kpp_rejects_wrong_length() {
    assert!(parse_cell("123", CellParser::Kpp).is_err());
}

#[test]
fn parse_boolean_ru() {
    assert_eq!(parse_cell("да", CellParser::Boolean).unwrap(), "true");
    assert_eq!(parse_cell("нет", CellParser::Boolean).unwrap(), "false");
}

#[test]
fn parse_boolean_en() {
    assert_eq!(parse_cell("yes", CellParser::Boolean).unwrap(), "true");
    assert_eq!(parse_cell("no", CellParser::Boolean).unwrap(), "false");
}

#[test]
fn parse_person_name_splits() {
    assert_eq!(
        parse_cell("Иванов Иван Иванович", CellParser::PersonName).unwrap(),
        "Иванов|Иван|Иванович"
    );
}

#[test]
fn parse_person_name_two_parts() {
    assert_eq!(
        parse_cell("Иванов Иван", CellParser::PersonName).unwrap(),
        "Иванов|Иван|"
    );
}

#[test]
fn parse_integer_valid() {
    assert_eq!(parse_cell("42", CellParser::Integer).unwrap(), "42");
}

#[test]
fn parse_integer_rejects_non_number() {
    assert!(parse_cell("abc", CellParser::Integer).is_err());
}

#[test]
fn parse_decimal_comma_to_dot() {
    assert_eq!(parse_cell("3,14", CellParser::Decimal).unwrap(), "3.14");
}

#[test]
fn parse_empty_cell_returns_empty() {
    assert_eq!(parse_cell("", CellParser::Text).unwrap(), "");
    assert_eq!(parse_cell("", CellParser::Inn).unwrap(), "");
    assert_eq!(parse_cell("", CellParser::Date).unwrap(), "");
}
