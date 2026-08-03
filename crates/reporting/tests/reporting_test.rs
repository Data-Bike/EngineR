//! Reporting crate tests (docs/15 §8-11).

#![cfg(test)]

use enginer_reporting::{
    escape_formula_injection, Dataset, DatasetColumn, DatasetColumnType, DatasetQuery,
    DatasetSnapshot, DatasetStatus, ExportFormat, FieldFormat, RenderContext, RenderOutput,
    RenderStatus, ReportAudience, ReportDefinition, ReportSection, ReportStatus,
    SummaryDefinition, TableDefinition, TemplateDefinition, TemplateFormat, TemplateLocale,
    TemplateVersion,
};

// --- Dataset tests ---

fn make_dataset() -> Dataset {
    Dataset {
        dataset_id: "ds-001".to_string(),
        version: "1.0.0".to_string(),
        name: "Legal entities".to_string(),
        description_key: "dataset.legal_entities".to_string(),
        columns: vec![
            DatasetColumn {
                name: "full_name".to_string(),
                label_key: "column.full_name".to_string(),
                column_type: DatasetColumnType::Text,
                nullable: false,
                classification: None,
                format_pattern: None,
            },
            DatasetColumn {
                name: "inn".to_string(),
                label_key: "column.inn".to_string(),
                column_type: DatasetColumnType::Text,
                nullable: false,
                classification: Some("restricted".to_string()),
                format_pattern: None,
            },
        ],
        query: DatasetQuery {
            query_type: "entity_list".to_string(),
            target_type: "legal_entity".to_string(),
            filter: None,
            group_by: vec![],
            order_by: vec![("full_name".to_string(), true)],
            limit: Some(1000),
            parameters: vec![],
        },
        status: DatasetStatus::Active,
        owner: "team-001".to_string(),
        requires_snapshot: true,
    }
}

#[test]
fn dataset_snapshot_new() {
    let snapshot = DatasetSnapshot::new("ds-001", "1.0.0", "srev-001", "prev-001");
    assert_eq!(snapshot.status, DatasetStatus::Materializing);
    assert!(!snapshot.is_ready());
}

#[test]
fn dataset_snapshot_mark_ready() {
    let mut snapshot = DatasetSnapshot::new("ds-001", "1.0.0", "srev-001", "prev-001");
    snapshot.mark_ready(500, "hash123", "s3://bucket/snapshot.parquet");
    assert_eq!(snapshot.status, DatasetStatus::Ready);
    assert!(snapshot.is_ready());
    assert_eq!(snapshot.row_count, 500);
}

#[test]
fn dataset_snapshot_mark_stale() {
    let mut snapshot = DatasetSnapshot::new("ds-001", "1.0.0", "srev-001", "prev-001");
    snapshot.mark_ready(100, "hash", "url");
    snapshot.mark_stale();
    assert_eq!(snapshot.status, DatasetStatus::Stale);
    assert!(!snapshot.is_ready());
}

// --- Export format tests ---

#[test]
fn export_format_extension() {
    assert_eq!(ExportFormat::Xlsx.extension(), "xlsx");
    assert_eq!(ExportFormat::Csv.extension(), "csv");
    assert_eq!(ExportFormat::Pdf.extension(), "pdf");
    assert_eq!(ExportFormat::Docx.extension(), "docx");
    assert_eq!(ExportFormat::Ndjson.extension(), "ndjson");
    assert_eq!(ExportFormat::Json.extension(), "json");
}

#[test]
fn export_format_mime_type() {
    assert_eq!(ExportFormat::Pdf.mime_type(), "application/pdf");
    assert_eq!(ExportFormat::Csv.mime_type(), "text/csv");
}

// --- Report definition tests ---

fn make_report_definition() -> ReportDefinition {
    ReportDefinition {
        definition_id: "report-001".to_string(),
        version: "1.0.0".to_string(),
        name: "Legal entity dossier".to_string(),
        description_key: "report.legal_entity_dossier".to_string(),
        dataset_ids: vec!["ds-001".to_string()],
        sections: vec![ReportSection {
            name: "main".to_string(),
            label_key: "section.main".to_string(),
            order: 1,
            tables: vec![TableDefinition {
                name: "entities".to_string(),
                label_key: "table.entities".to_string(),
                dataset_id: "ds-001".to_string(),
                columns: vec!["full_name".to_string(), "inn".to_string()],
                row_limit: Some(100),
                show_totals: false,
            }],
            charts: vec![],
            summaries: vec![SummaryDefinition {
                name: "total".to_string(),
                label_key: "summary.total".to_string(),
                dataset_id: "ds-001".to_string(),
                aggregation: "count".to_string(),
                column: None,
                filter: None,
            }],
        }],
        default_locale: "ru".to_string(),
        default_timezone: "Europe/Moscow".to_string(),
        template_id: "tpl-001".to_string(),
        template_version: "1.0.0".to_string(),
        allowed_formats: vec![ExportFormat::Pdf, ExportFormat::Xlsx],
        page_limit: Some(50),
        row_limit: Some(1000),
        confidentiality: "restricted".to_string(),
        watermark: true,
        allowed_audiences: vec![
            ReportAudience {
                audience_type: "group".to_string(),
                audience_id: "risk-team".to_string(),
                can_reexport: false,
            },
        ],
        requires_snapshot: true,
        status: ReportStatus::Active,
        owner: "team-001".to_string(),
        schedule: None,
    }
}

#[test]
fn report_definition_is_format_allowed() {
    let def = make_report_definition();
    assert!(def.is_format_allowed(ExportFormat::Pdf));
    assert!(def.is_format_allowed(ExportFormat::Xlsx));
    assert!(!def.is_format_allowed(ExportFormat::Csv));
}

#[test]
fn report_definition_is_audience_allowed() {
    let def = make_report_definition();
    assert!(def.is_audience_allowed("group", "risk-team"));
    assert!(!def.is_audience_allowed("group", "other-team"));
    assert!(!def.is_audience_allowed("user", "user-001"));
}

#[test]
fn report_definition_all_tables() {
    let def = make_report_definition();
    let tables = def.all_tables();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "entities");
}

#[test]
fn report_definition_all_summaries() {
    let def = make_report_definition();
    let summaries = def.all_summaries();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].aggregation, "count");
}

// --- Template tests ---

fn make_template() -> TemplateDefinition {
    TemplateDefinition {
        template_id: "tpl-001".to_string(),
        name: "Legal entity dossier template".to_string(),
        description_key: "template.dossier".to_string(),
        format: TemplateFormat::Docx,
        versions: vec![TemplateVersion {
            version: "1.0.0".to_string(),
            format: TemplateFormat::Docx,
            created_at: chrono::Utc::now(),
            current: true,
            locales: vec![
                TemplateLocale {
                    locale: "ru".to_string(),
                    storage_url: "s3://templates/tpl-001/ru/1.0.0.docx".to_string(),
                    content_hash: "hash-ru".to_string(),
                },
                TemplateLocale {
                    locale: "en".to_string(),
                    storage_url: "s3://templates/tpl-001/en/1.0.0.docx".to_string(),
                    content_hash: "hash-en".to_string(),
                },
            ],
            field_formats: vec![FieldFormat {
                field_name: "inn".to_string(),
                pattern: "0000000000".to_string(),
                masked: false,
                mask_pattern: None,
            }],
        }],
        allowed_datasets: vec!["ds-001".to_string()],
        has_watermark: true,
    }
}

#[test]
fn template_current_version() {
    let tpl = make_template();
    let current = tpl.current_version().unwrap();
    assert_eq!(current.version, "1.0.0");
    assert!(current.current);
}

#[test]
fn template_current_locale() {
    let tpl = make_template();
    let locale = tpl.current_locale("ru").unwrap();
    assert_eq!(locale.locale, "ru");
    assert_eq!(locale.content_hash, "hash-ru");
}

#[test]
fn template_is_dataset_allowed() {
    let tpl = make_template();
    assert!(tpl.is_dataset_allowed("ds-001"));
    assert!(!tpl.is_dataset_allowed("ds-999"));
}

#[test]
fn template_add_version() {
    let mut tpl = make_template();
    let new_version = TemplateVersion {
        version: "2.0.0".to_string(),
        format: TemplateFormat::Docx,
        created_at: chrono::Utc::now(),
        current: true,
        locales: vec![TemplateLocale {
            locale: "ru".to_string(),
            storage_url: "s3://templates/tpl-001/ru/2.0.0.docx".to_string(),
            content_hash: "hash-ru-v2".to_string(),
        }],
        field_formats: vec![],
    };
    tpl.add_version(new_version);
    assert_eq!(tpl.versions.len(), 2);
    assert_eq!(tpl.current_version().unwrap().version, "2.0.0");
    // Old version should no longer be current
    assert!(!tpl.versions[0].current);
}

#[test]
fn template_get_version() {
    let tpl = make_template();
    let v = tpl.get_version("1.0.0").unwrap();
    assert_eq!(v.version, "1.0.0");
    assert!(tpl.get_version("999").is_none());
}

// --- Render tests ---

#[test]
fn render_output_pending() {
    let output = RenderOutput::pending(
        "report-001",
        "1.0.0",
        "1.0.0",
        "srev-001",
        "prev-001",
        ExportFormat::Pdf,
    );
    assert_eq!(output.status, RenderStatus::Pending);
    assert!(!output.is_terminal());
}

#[test]
fn render_output_mark_completed() {
    let mut output = RenderOutput::pending(
        "report-001",
        "1.0.0",
        "1.0.0",
        "srev-001",
        "prev-001",
        ExportFormat::Pdf,
    );
    output.mark_completed(
        "s3://output/report.pdf",
        "hash123",
        1024 * 1024,
        Some(10),
        500,
        3000,
    );
    assert_eq!(output.status, RenderStatus::Completed);
    assert!(output.is_terminal());
    assert_eq!(output.page_count, Some(10));
    assert_eq!(output.row_count, 500);
}

#[test]
fn render_output_mark_failed() {
    let mut output = RenderOutput::pending(
        "report-001",
        "1.0.0",
        "1.0.0",
        "srev-001",
        "prev-001",
        ExportFormat::Pdf,
    );
    output.mark_failed();
    assert_eq!(output.status, RenderStatus::Failed);
    assert!(output.is_terminal());
}

#[test]
fn render_context() {
    let ctx = RenderContext {
        tenant_id: "tenant-001".to_string(),
        actor: "user-001".to_string(),
        locale: "ru".to_string(),
        timezone: "Europe/Moscow".to_string(),
        format: ExportFormat::Pdf,
        scheduled: false,
        snapshot_grant_id: None,
        watermark: true,
        encrypt: false,
        sign: true,
        classification: "restricted".to_string(),
    };
    assert_eq!(ctx.locale, "ru");
    assert!(!ctx.scheduled);
    assert!(ctx.sign);
}

// --- Formula injection escape tests ---

#[test]
fn escape_formula_injection_equal() {
    assert_eq!(escape_formula_injection("=SUM(A1:A10)"), "'=SUM(A1:A10)");
}

#[test]
fn escape_formula_injection_plus() {
    assert_eq!(escape_formula_injection("+123"), "'+123");
}

#[test]
fn escape_formula_injection_minus() {
    assert_eq!(escape_formula_injection("-123"), "'-123");
}

#[test]
fn escape_formula_injection_at() {
    assert_eq!(escape_formula_injection("@cmd"), "'@cmd");
}

#[test]
fn escape_formula_injection_safe_text() {
    assert_eq!(escape_formula_injection("hello world"), "hello world");
}

#[test]
fn escape_formula_injection_safe_number() {
    assert_eq!(escape_formula_injection("12345"), "12345");
}

#[test]
fn escape_formula_injection_leading_spaces() {
    assert_eq!(escape_formula_injection("  =evil"), "'  =evil");
}
