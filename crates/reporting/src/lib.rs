//! Reporting: datasets, report definitions, template rendering
//! (docs/15 §8-11).
//!
//! ReportDefinition: dataset query, sections/tables/charts/summaries,
//! locale/timezone, DOCX/XLSX/PDF template version, field formatting,
//! confidentiality/watermark, page/row limits, allowed audiences/formats,
//! snapshot consistency, schedule/delivery (docs/15 §9).
//!
//! Template does not contain SQL and cannot access arbitrary object path
//! outside declared dataset (docs/15 §9).

#![forbid(unsafe_code)]

pub mod dataset;
pub mod definition;
pub mod error;
pub mod render;
pub mod template;

pub use dataset::{
    Dataset, DatasetColumn, DatasetColumnType, DatasetQuery, DatasetSnapshot, DatasetStatus,
};
pub use definition::{
    ChartDefinition, ExportFormat, ReportAudience, ReportDefinition, ReportSection,
    ReportStatus, SummaryDefinition, TableDefinition,
};
pub use error::{ReportError, ReportResult};
pub use render::{
    escape_formula_injection, RenderContext, RenderOutput, RenderResult, RenderStatus,
    TemplateRenderer,
};
pub use template::{
    FieldFormat, Template, TemplateDefinition, TemplateFormat, TemplateLocale,
    TemplateVersion,
};
