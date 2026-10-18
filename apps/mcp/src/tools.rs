//! MCP tool catalog — declarative mapping of EngineR API endpoints to MCP
//! tools. Each tool knows the HTTP method, the path template (`{param}`
//! placeholders), the parameter location (path/query/body) and the EngineR
//! capability that gates it. `tools/list` only advertises tools the caller's
//! token is actually permitted to use; `tools/call` still relies on the API's
//! own PDP enforcement (fail closed).

use serde_json::{json, Map, Value};

/// Where a tool argument lands on the outbound HTTP request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLoc {
    /// Substituted into `{name}` placeholders of the path template.
    Path,
    /// Sent as a URL query parameter (GET/DELETE) — ignored for body methods.
    Query,
    /// Member of the JSON request body (POST/PUT).
    Body,
}

/// One MCP-callable parameter of a tool.
pub struct Param {
    pub name: &'static str,
    pub loc: ParamLoc,
    /// JSON Schema `type` value: string|integer|number|boolean|object|array.
    pub ty: &'static str,
    pub required: bool,
    pub desc: &'static str,
}

/// Declarative tool definition bound to an EngineR API endpoint.
pub struct ToolDef {
    /// MCP tool name (`domain.action`).
    pub name: &'static str,
    pub desc: &'static str,
    /// HTTP verb used against the EngineR API.
    pub method: &'static str,
    /// `/api/v1`-relative path template; `{name}` segments are filled from
    /// `ParamLoc::Path` arguments.
    pub path: &'static str,
    /// Capability wire string from `enginer_security_domain::CapabilityAction`.
    /// `None` = no capability needed beyond a valid credential.
    pub capability: Option<&'static str>,
    pub params: &'static [Param],
}

const P: ParamLoc = ParamLoc::Path;
const Q: ParamLoc = ParamLoc::Query;
const B: ParamLoc = ParamLoc::Body;

/// Entity primary identifier used in path templates (public id / uuid).
const ID: Param = Param {
    name: "id",
    loc: P,
    ty: "string",
    required: true,
    desc: "Entity public id or task/check/group/user id (uuid).",
};

/// Stable alias identifier (types, predicates) used in path templates.
const ALIAS: Param = Param {
    name: "alias",
    loc: P,
    ty: "string",
    required: true,
    desc: "Stable alias, e.g. legal_entity or owns.",
};

/// Optimistic-locking precondition — lifted out of the body into the
/// `If-Match` header by `prepare_call`.
const IF_MATCH: Param = Param {
    name: "if_match",
    loc: B,
    ty: "string",
    required: false,
    desc: "ETag from a prior read — sent as the required If-Match header.",
};

#[rustfmt::skip]
pub static TOOLS: &[ToolDef] = &[
    // --- Identity / session context -------------------------------------
    ToolDef {
        name: "context.get",
        desc: "Current principal context: identity, tenant, capabilities, URL rules.",
        method: "GET", path: "/me/context", capability: None, params: &[],
    },
    ToolDef {
        name: "dashboard.stats",
        desc: "Aggregate dashboard counters for the tenant.",
        method: "GET", path: "/dashboard/stats", capability: Some("entity.read"), params: &[],
    },

    // --- Entities ---------------------------------------------------------
    ToolDef {
        name: "entities.list",
        desc: "List/search entities. Filter by type alias and free-text search.",
        method: "GET", path: "/entities", capability: Some("entity.read"),
        params: &[
            Param { name: "type", loc: Q, ty: "string", required: false, desc: "Type alias, e.g. legal_entity." },
            Param { name: "search", loc: Q, ty: "string", required: false, desc: "Free-text search." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size (default server-side)." },
            Param { name: "cursor", loc: Q, ty: "string", required: false, desc: "Pagination cursor." },
        ],
    },
    ToolDef {
        name: "entities.get",
        desc: "Get a single entity with its statements and attributes.",
        method: "GET", path: "/entities/{id}", capability: Some("entity.read"),
        params: &[ID],
    },
    ToolDef {
        name: "entities.create",
        desc: "Create an entity. `type_id` selects the schema type (uuid or alias); `fields` carries predicate values.",
        method: "POST", path: "/entities", capability: Some("entity.create"),
        params: &[
            Param { name: "type_id", loc: B, ty: "string", required: true, desc: "Published type alias or uuid." },
            Param { name: "fields", loc: B, ty: "object", required: false, desc: "Predicate values keyed by predicate alias." },
            Param { name: "statements", loc: B, ty: "array", required: false, desc: "Explicit statements (predicate_id, kind, value)." },
            Param { name: "identity_mode", loc: B, ty: "string", required: false, desc: "Identity conflict mode: deduplicate (default)|reject|create_candidate." },
            Param { name: "schema_revision", loc: B, ty: "integer", required: false, desc: "Pin to a published schema revision." },
        ],
    },
    ToolDef {
        name: "entities.update",
        desc: "Update entity fields (predicate values).",
        method: "PUT", path: "/entities/{id}", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "fields", loc: B, ty: "object", required: false, desc: "Predicate values to set (alias→value map; a statement array also works)." },
            Param { name: "predicate_id", loc: B, ty: "string", required: false, desc: "Single predicate to set (with `value`)." },
            Param { name: "value", loc: B, ty: "object", required: false, desc: "Value for `predicate_id`." },
            Param { name: "revision", loc: B, ty: "integer", required: false, desc: "Expected revision (optimistic concurrency)." },
        ],
    },
    ToolDef {
        name: "entities.retire",
        desc: "Retire (soft-delete) an entity.",
        method: "POST", path: "/entities/{id}/retire", capability: Some("entity.retire"),
        params: &[
            ID,
            Param { name: "reason", loc: B, ty: "string", required: false, desc: "Retirement reason." },
        ],
    },
    ToolDef {
        name: "entities.merge",
        desc: "Merge another entity into this one.",
        method: "POST", path: "/entities/{id}/merge", capability: Some("entity.merge"),
        params: &[
            ID,
            Param { name: "loser_public_id", loc: B, ty: "string", required: true, desc: "Public id of the entity merged away." },
            Param { name: "reason", loc: B, ty: "string", required: false, desc: "Merge reason." },
        ],
    },
    ToolDef {
        name: "entities.query",
        desc: "Run an advanced entity query (AST filters, facets, sort).",
        method: "POST", path: "/entities/query", capability: Some("query.advanced"),
        params: &[
            Param { name: "query", loc: B, ty: "object", required: true, desc: "Query AST per docs/12." },
        ],
    },
    ToolDef {
        name: "entities.graph",
        desc: "Entity neighbourhood graph (relations up to a depth/direction).",
        method: "POST", path: "/entities/{id}/graph", capability: Some("entity.read"),
        params: &[
            ID,
            Param { name: "max_depth", loc: B, ty: "integer", required: false, desc: "Traversal depth." },
            Param { name: "direction", loc: B, ty: "string", required: false, desc: "outgoing|incoming|both." },
            Param { name: "locale", loc: B, ty: "string", required: false, desc: "Label locale." },
            Param { name: "predicate_id", loc: B, ty: "string", required: false, desc: "Restrict to one relation predicate." },
        ],
    },
    ToolDef {
        name: "entities.history",
        desc: "Revision history of an entity.",
        method: "GET", path: "/entities/{id}/history", capability: Some("entity.read"),
        params: &[ID],
    },

    // --- Relations --------------------------------------------------------
    ToolDef {
        name: "relations.list",
        desc: "List relations of an entity (direction-aware).",
        method: "GET", path: "/entities/{id}/relations", capability: Some("entity.read"),
        params: &[
            ID,
            Param { name: "direction", loc: Q, ty: "string", required: false, desc: "outgoing|incoming|both." },
            Param { name: "as_of", loc: Q, ty: "string", required: false, desc: "RFC3339 timestamp." },
        ],
    },
    ToolDef {
        name: "relations.create",
        desc: "Create a relation between this entity and a target.",
        method: "POST", path: "/entities/{id}/relations", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "predicate", loc: B, ty: "string", required: true, desc: "Predicate alias." },
            Param { name: "target_entity_id", loc: B, ty: "string", required: false, desc: "Target entity public id (one of target_* required)." },
            Param { name: "target_public_id", loc: B, ty: "string", required: false, desc: "Target entity public id (alternative)." },
        ],
    },
    ToolDef {
        name: "relations.delete",
        desc: "Delete a relation instance of an entity.",
        method: "DELETE", path: "/entities/{id}/relations/{relation_id}", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "relation_id", loc: P, ty: "string", required: true, desc: "Relation instance id." },
        ],
    },

    // --- Documents ---------------------------------------------------------
    ToolDef {
        name: "documents.list",
        desc: "List documents attached to an entity.",
        method: "GET", path: "/entities/{id}/documents", capability: Some("entity.read"),
        params: &[ID],
    },
    ToolDef {
        name: "documents.upload",
        desc: "Attach a document to an entity (small files; bytes is a JSON array of 0-255 ints).",
        method: "POST", path: "/entities/{id}/documents", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "kind", loc: B, ty: "string", required: true, desc: "Document kind." },
            Param { name: "file_name", loc: B, ty: "string", required: true, desc: "File name." },
            Param { name: "bytes", loc: B, ty: "array", required: true, desc: "File content as JSON array of byte values." },
            Param { name: "title", loc: B, ty: "string", required: true, desc: "Display title (send file_name if none)." },
            Param { name: "content_type", loc: B, ty: "string", required: true, desc: "MIME type." },
        ],
    },
    ToolDef {
        name: "documents.delete",
        desc: "Remove a document from an entity.",
        method: "DELETE", path: "/entities/{id}/documents/{doc_id}", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "doc_id", loc: P, ty: "string", required: true, desc: "Document id." },
        ],
    },
    // --- Files (staged upload is binary-chunked — only metadata ops here) -----
    ToolDef {
        name: "files.get",
        desc: "File object metadata (content hash, size, status).",
        method: "GET", path: "/files/{id}", capability: Some("entity.edit"),
        params: &[ID],
    },
    ToolDef {
        name: "files.delete",
        desc: "Delete a file object.",
        method: "DELETE", path: "/files/{id}", capability: Some("entity.edit"),
        params: &[ID],
    },
    ToolDef {
        name: "files.upload_init",
        desc: "Begin a staged upload session (chunk upload itself is binary and not exposed over MCP).",
        method: "POST", path: "/files/upload/init", capability: Some("entity.edit"),
        params: &[
            Param { name: "filename", loc: B, ty: "string", required: true, desc: "File name." },
            Param { name: "expected_size", loc: B, ty: "integer", required: true, desc: "Bytes total." },
            Param { name: "expected_hash", loc: B, ty: "string", required: true, desc: "Hex digest of the full content." },
            Param { name: "chunk_size", loc: B, ty: "integer", required: true, desc: "Chunk size in bytes." },
            Param { name: "hash_algorithm", loc: B, ty: "string", required: false, desc: "sha256|blake3 (default sha256)." },
            Param { name: "content_type", loc: B, ty: "string", required: false, desc: "MIME type." },
        ],
    },
    ToolDef {
        name: "files.upload_status",
        desc: "Staged upload session status.",
        method: "GET", path: "/files/upload/{id}", capability: Some("entity.edit"),
        params: &[ID],
    },
    ToolDef {
        name: "files.upload_abort",
        desc: "Abort a staged upload session.",
        method: "POST", path: "/files/upload/{id}/abort", capability: Some("entity.edit"),
        params: &[ID],
    },
    ToolDef {
        name: "files.upload_chunk",
        desc: "Upload one chunk of a staged upload (data_base64 = base64-encoded raw bytes).",
        method: "PUT", path: "/files/upload/{id}/chunks/{index}", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "index", loc: P, ty: "integer", required: true, desc: "Chunk index." },
            Param { name: "data_base64", loc: B, ty: "string", required: true, desc: "Base64-encoded chunk bytes." },
        ],
    },
    ToolDef {
        name: "files.upload_complete",
        desc: "Finish a staged upload — verifies hash/size and marks the file ready.",
        method: "POST", path: "/files/upload/{id}/complete", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "final_hash", loc: B, ty: "string", required: true, desc: "Hex digest of the full content." },
            Param { name: "total_size", loc: B, ty: "integer", required: true, desc: "Bytes total." },
        ],
    },
    ToolDef {
        name: "files.download",
        desc: "Download a file object — response returned as base64.",
        method: "GET", path: "/files/{id}/download", capability: Some("entity.edit"),
        params: &[ID],
    },
    ToolDef {
        name: "documents.download",
        desc: "Download an entity document — response returned as base64.",
        method: "GET", path: "/entities/{id}/documents/{doc_id}/download", capability: Some("entity.read"),
        params: &[
            ID,
            Param { name: "doc_id", loc: P, ty: "string", required: true, desc: "Document id." },
        ],
    },

    // --- Tasks --------------------------------------------------------------
    ToolDef {
        name: "tasks.list",
        desc: "List workflow tasks attached to an entity.",
        method: "GET", path: "/entities/{id}/tasks", capability: Some("entity.read"),
        params: &[ID],
    },
    ToolDef {
        name: "tasks.create",
        desc: "Create a task on an entity.",
        method: "POST", path: "/entities/{id}/tasks", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "kind", loc: B, ty: "string", required: true, desc: "Task kind." },
            Param { name: "title", loc: B, ty: "string", required: false, desc: "Task title." },
            Param { name: "description", loc: B, ty: "string", required: false, desc: "Task description." },
            Param { name: "priority", loc: B, ty: "string", required: false, desc: "Task priority." },
            Param { name: "due_date", loc: B, ty: "string", required: false, desc: "RFC3339 due date." },
        ],
    },
    ToolDef {
        name: "tasks.start",
        desc: "Mark a task in-progress.",
        method: "POST", path: "/tasks/{id}/start", capability: Some("entity.edit"),
        params: &[ID],
    },
    ToolDef {
        name: "tasks.complete",
        desc: "Complete a task.",
        method: "POST", path: "/tasks/{id}/complete", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "resolution", loc: B, ty: "string", required: false, desc: "Resolution note." },
        ],
    },
    ToolDef {
        name: "tasks.cancel",
        desc: "Cancel a task.",
        method: "POST", path: "/tasks/{id}/cancel", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "reason", loc: B, ty: "string", required: false, desc: "Cancellation reason." },
        ],
    },

    // --- Risk / assessments --------------------------------------------------
    ToolDef {
        name: "factors.list",
        desc: "Risk factors of an entity.",
        method: "GET", path: "/entities/{id}/factors", capability: Some("risk.view"),
        params: &[ID],
    },
    ToolDef {
        name: "factors.resolve",
        desc: "Resolve a risk factor on an entity.",
        method: "POST", path: "/entities/{id}/factors/{factor_id}/resolve", capability: Some("risk.resolve"),
        params: &[
            ID,
            Param { name: "factor_id", loc: P, ty: "string", required: true, desc: "Factor id." },
            Param { name: "resolution", loc: B, ty: "string", required: false, desc: "Resolution note." },
        ],
    },
    ToolDef {
        name: "assessment.get",
        desc: "Current risk assessment of an entity.",
        method: "GET", path: "/entities/{id}/assessment", capability: Some("assessment.view"),
        params: &[ID],
    },
    ToolDef {
        name: "assessment.create",
        desc: "Trigger a new risk assessment for an entity.",
        method: "POST", path: "/entities/{id}/assessment", capability: Some("assessment.view"),
        params: &[
            ID,
            Param { name: "rationale", loc: B, ty: "string", required: false, desc: "Analyst rationale." },
            Param { name: "desired_level", loc: B, ty: "string", required: false, desc: "minimal|low|medium|high|critical." },
            Param { name: "decision", loc: B, ty: "string", required: false, desc: "accept|reject|escalate|review." },
            Param { name: "force", loc: B, ty: "boolean", required: false, desc: "Force a new assessment even if one exists." },
        ],
    },
    ToolDef {
        name: "assessments.list",
        desc: "List risk assessments of the tenant.",
        method: "GET", path: "/assessments", capability: Some("assessment.view"),
        params: &[
            Param { name: "status", loc: Q, ty: "string", required: false, desc: "Status filter." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size." },
            Param { name: "cursor", loc: Q, ty: "string", required: false, desc: "Pagination cursor." },
        ],
    },

    // --- Checks --------------------------------------------------------------
    ToolDef {
        name: "checks.list",
        desc: "List check instances.",
        method: "GET", path: "/checks", capability: Some("check.view"),
        params: &[
            Param { name: "target_entity_id", loc: Q, ty: "string", required: false, desc: "Filter by entity." },
            Param { name: "status", loc: Q, ty: "string", required: false, desc: "pending|running|completed|failed." },
            Param { name: "kind", loc: Q, ty: "string", required: false, desc: "Check kind." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size." },
            Param { name: "cursor", loc: Q, ty: "string", required: false, desc: "Pagination cursor." },
        ],
    },
    ToolDef {
        name: "checks.get",
        desc: "Get a check instance.",
        method: "GET", path: "/checks/{id}", capability: Some("check.view"),
        params: &[ID],
    },
    ToolDef {
        name: "checks.evidence",
        desc: "Evidence items of a check instance.",
        method: "GET", path: "/checks/{id}/evidence", capability: Some("check.view"),
        params: &[ID],
    },
    ToolDef {
        name: "checks.create",
        desc: "Request a new check for an entity.",
        method: "POST", path: "/checks", capability: Some("check.run"),
        params: &[
            Param { name: "target_entity_id", loc: B, ty: "string", required: true, desc: "Entity to check." },
            Param { name: "check_kind", loc: B, ty: "string", required: true, desc: "kyc|sanctions|financial_health|…" },
            Param { name: "request_id", loc: B, ty: "string", required: false, desc: "Originating request id." },
        ],
    },
    ToolDef {
        name: "entity_checks.list",
        desc: "Checks attached to a specific entity.",
        method: "GET", path: "/entities/{id}/checks", capability: Some("check.view"),
        params: &[ID],
    },

    // --- Schema administration -------------------------------------------------
    ToolDef {
        name: "schema.types",
        desc: "Published entity types (permission-filtered).",
        method: "GET", path: "/schemas/types", capability: Some("entity.read"), params: &[],
    },
    ToolDef {
        name: "schema.type_get",
        desc: "Type detail with predicate slots.",
        method: "GET", path: "/schemas/types/{alias}", capability: Some("entity.read"),
        params: &[ALIAS],
    },
    ToolDef {
        name: "schema.type_view",
        desc: "Resolved view definition (form|list) for a type — fields are permission-filtered.",
        method: "GET", path: "/schemas/types/{alias}/view", capability: Some("entity.read"),
        params: &[
            ALIAS,
            Param { name: "view_type", loc: Q, ty: "string", required: true, desc: "form|list." },
        ],
    },
    ToolDef {
        name: "schema.predicates",
        desc: "Published predicates (relation predicates are permission-filtered).",
        method: "GET", path: "/schemas/predicates", capability: Some("entity.read"), params: &[],
    },
    ToolDef {
        name: "schema.published",
        desc: "Published schema revisions.",
        method: "GET", path: "/schemas/published", capability: Some("entity.read"), params: &[],
    },
    ToolDef {
        name: "schema.drafts",
        desc: "List schema drafts.",
        method: "GET", path: "/schemas/drafts", capability: Some("schema.draft"), params: &[],
    },
    ToolDef {
        name: "schema.draft_get",
        desc: "Get a schema draft.",
        method: "GET", path: "/schemas/drafts/{id}", capability: Some("schema.draft"),
        params: &[ID],
    },
    ToolDef {
        name: "schema.draft_create",
        desc: "Create a schema draft.",
        method: "POST", path: "/schemas/drafts", capability: Some("schema.draft"),
        params: &[
            Param { name: "name", loc: B, ty: "string", required: true, desc: "Draft name." },
            Param { name: "base_revision", loc: B, ty: "string", required: false, desc: "Revision to branch from." },
        ],
    },
    ToolDef {
        name: "schema.draft_update",
        desc: "Update a schema draft (types/predicates/changes).",
        method: "PUT", path: "/schemas/drafts/{id}", capability: Some("schema.draft"),
        params: &[
            ID,
            Param { name: "display_name", loc: B, ty: "string", required: true, desc: "Draft display name." },
            Param { name: "semantic_version", loc: B, ty: "string", required: true, desc: "Semver of the draft." },
            Param { name: "description", loc: B, ty: "string", required: false, desc: "Draft description." },
            Param { name: "types", loc: B, ty: "array", required: false, desc: "Type definitions." },
            Param { name: "predicates", loc: B, ty: "array", required: false, desc: "Predicate definitions." },
            Param { name: "changes", loc: B, ty: "array", required: false, desc: "Schema change entries." },
            IF_MATCH,
        ],
    },
    ToolDef {
        name: "schema.draft_delete",
        desc: "Delete a schema draft.",
        method: "DELETE", path: "/schemas/drafts/{id}", capability: Some("schema.draft"),
        params: &[ID, IF_MATCH],
    },
    ToolDef {
        name: "schema.draft_validate",
        desc: "Validate a schema draft against the registry rules.",
        method: "POST", path: "/schemas/drafts/{id}/validate", capability: Some("schema.draft"),
        params: &[ID],
    },
    ToolDef {
        name: "schema.draft_impact",
        desc: "Impact analysis of publishing a draft.",
        method: "POST", path: "/schemas/drafts/{id}/impact", capability: Some("schema.draft"),
        params: &[ID],
    },
    ToolDef {
        name: "schema.draft_publish",
        desc: "Publish a schema draft (creates a new revision).",
        method: "POST", path: "/schemas/drafts/{id}/publish", capability: Some("schema.publish"),
        params: &[
            ID,
            Param { name: "reason", loc: B, ty: "string", required: true, desc: "Publish reason (audit)." },
            IF_MATCH,
        ],
    },
    ToolDef {
        name: "schema.revision_get",
        desc: "Get a published schema revision.",
        method: "GET", path: "/schemas/revisions/{id}", capability: Some("entity.read"),
        params: &[ID],
    },
    ToolDef {
        name: "schema.revision_retire",
        desc: "Retire a published schema revision.",
        method: "POST", path: "/schemas/revisions/{id}/retire", capability: Some("schema.publish"),
        params: &[
            ID,
            Param { name: "reason", loc: B, ty: "string", required: false, desc: "Retirement reason." },
            IF_MATCH,
        ],
    },
    ToolDef {
        name: "schema.revision_migration",
        desc: "Migration status of a published revision.",
        method: "GET", path: "/schemas/revisions/{id}/migration-status", capability: Some("entity.read"),
        params: &[ID],
    },

    // --- Requests / workflows ---------------------------------------------------
    ToolDef {
        name: "requests.list",
        desc: "List requests/processes.",
        method: "GET", path: "/requests", capability: Some("entity.read"),
        params: &[
            Param { name: "status", loc: Q, ty: "string", required: false, desc: "Status filter." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size." },
            Param { name: "cursor", loc: Q, ty: "string", required: false, desc: "Pagination cursor." },
        ],
    },
    ToolDef {
        name: "requests.create",
        desc: "Create a request/process.",
        method: "POST", path: "/requests", capability: Some("entity.create"),
        params: &[
            Param { name: "target_entity_id", loc: B, ty: "string", required: true, desc: "Entity the request is about." },
            Param { name: "check_kinds", loc: B, ty: "array", required: false, desc: "Check kinds to run." },
            Param { name: "requested_product", loc: B, ty: "string", required: true, desc: "Requested product." },
            Param { name: "requested_limit", loc: B, ty: "number", required: false, desc: "Requested limit." },
            Param { name: "urgency", loc: B, ty: "string", required: false, desc: "Urgency level." },
            Param { name: "due_date", loc: B, ty: "string", required: false, desc: "RFC3339 due date." },
        ],
    },
    ToolDef {
        name: "workflows.get",
        desc: "Workflow instance detail.",
        method: "GET", path: "/workflows/{id}", capability: Some("entity.read"),
        params: &[ID],
    },
    ToolDef {
        name: "workflows.tasks",
        desc: "Tasks of a workflow instance.",
        method: "GET", path: "/workflows/{id}/tasks", capability: Some("entity.read"),
        params: &[ID],
    },
    ToolDef {
        name: "workflows.create",
        desc: "Start a workflow instance.",
        method: "POST", path: "/workflows", capability: Some("entity.create"),
        params: &[
            Param { name: "definition_id", loc: B, ty: "string", required: true, desc: "Workflow definition id/alias." },
            Param { name: "target_entity_id", loc: B, ty: "string", required: true, desc: "Subject entity (required by the API)." },
            Param { name: "variables", loc: B, ty: "object", required: false, desc: "Initial variables." },
        ],
    },
    ToolDef {
        name: "workflows.transition",
        desc: "Trigger a workflow transition.",
        method: "POST", path: "/workflows/{id}/transition", capability: Some("entity.edit"),
        params: &[
            ID,
            Param { name: "trigger", loc: B, ty: "string", required: true, desc: "Transition trigger (action name from the workflow definition)." },
            Param { name: "to", loc: B, ty: "string", required: true, desc: "Target state." },
        ],
    },

    // --- Monitoring / reports ------------------------------------------------------
    ToolDef {
        name: "monitors.list",
        desc: "Monitors with schedule and last result.",
        method: "GET", path: "/monitors", capability: Some("monitor.manage"), params: &[],
    },
    ToolDef {
        name: "monitors.run",
        desc: "Run a monitor now.",
        method: "POST", path: "/monitors/{id}/run", capability: Some("monitor.manage"),
        params: &[ID],
    },
    ToolDef {
        name: "reports.list",
        desc: "Report instances.",
        method: "GET", path: "/reports", capability: Some("report.view"), params: &[],
    },
    ToolDef {
        name: "reports.generate",
        desc: "Generate a report.",
        method: "POST", path: "/reports", capability: Some("report.run"),
        params: &[
            Param { name: "entity_id", loc: B, ty: "string", required: true, desc: "Entity the report covers." },
            Param { name: "template", loc: B, ty: "string", required: false, desc: "Report template id/alias." },
            Param { name: "format", loc: B, ty: "string", required: false, desc: "xlsx|csv|json|pdf|docx." },
        ],
    },

    // --- Connectors -------------------------------------------------------------------
    ToolDef {
        name: "connectors.list",
        desc: "Connector instances.",
        method: "GET", path: "/connectors", capability: Some("connector.manage"), params: &[],
    },
    ToolDef {
        name: "connectors.get",
        desc: "Connector instance detail.",
        method: "GET", path: "/connectors/{id}", capability: Some("connector.manage"),
        params: &[ID],
    },
    ToolDef {
        name: "connectors.update",
        desc: "Patch a connector instance (name, enabled flag, config keys).",
        method: "PATCH", path: "/connectors/{id}", capability: Some("connector.manage"),
        params: &[
            ID,
            Param { name: "display_name", loc: B, ty: "string", required: false, desc: "Instance name." },
            Param { name: "enabled", loc: B, ty: "boolean", required: false, desc: "Enable/disable." },
            Param { name: "environment", loc: B, ty: "string", required: false, desc: "prod|staging|…" },
            Param { name: "rate_limit_rps", loc: B, ty: "number", required: false, desc: "Max requests/sec." },
            Param { name: "trust_rank", loc: B, ty: "integer", required: false, desc: "0-100." },
        ],
    },
    ToolDef {
        name: "connectors.delete",
        desc: "Delete a connector instance.",
        method: "DELETE", path: "/connectors/{id}", capability: Some("connector.manage"),
        params: &[ID],
    },
    ToolDef {
        name: "connectors.create",
        desc: "Register a connector instance.",
        method: "POST", path: "/connectors", capability: Some("connector.manage"),
        params: &[
            Param { name: "connector_type_id", loc: B, ty: "string", required: true, desc: "Connector type id/alias." },
            Param { name: "display_name", loc: B, ty: "string", required: false, desc: "Instance name." },
            Param { name: "environment", loc: B, ty: "string", required: false, desc: "prod|staging|…" },
            Param { name: "rate_limit_rps", loc: B, ty: "number", required: false, desc: "Max requests/sec." },
            Param { name: "trust_rank", loc: B, ty: "integer", required: false, desc: "0-100 trust rank." },
        ],
    },
    ToolDef {
        name: "connectors.enable",
        desc: "Enable a connector instance.",
        method: "POST", path: "/connectors/{id}/enable", capability: Some("connector.manage"),
        params: &[ID],
    },
    ToolDef {
        name: "connectors.disable",
        desc: "Disable a connector instance.",
        method: "POST", path: "/connectors/{id}/disable", capability: Some("connector.manage"),
        params: &[ID],
    },
    ToolDef {
        name: "connectors.health",
        desc: "Run a health check on a connector instance.",
        method: "POST", path: "/connectors/{id}/health", capability: Some("connector.manage"),
        params: &[ID],
    },

    // --- Audit ---------------------------------------------------------------------------
    ToolDef {
        name: "audit.query",
        desc: "Query the audit trail (kind/actor/action/resource/effect/time filters).",
        method: "GET", path: "/audit", capability: Some("audit.read"),
        params: &[
            Param { name: "kind", loc: Q, ty: "string", required: false, desc: "Event kind." },
            Param { name: "actor", loc: Q, ty: "string", required: false, desc: "Actor principal id." },
            Param { name: "action", loc: Q, ty: "string", required: false, desc: "Action filter." },
            Param { name: "resource_kind", loc: Q, ty: "string", required: false, desc: "Resource kind." },
            Param { name: "resource_id", loc: Q, ty: "string", required: false, desc: "Resource id." },
            Param { name: "effect", loc: Q, ty: "string", required: false, desc: "allow|deny." },
            Param { name: "from", loc: Q, ty: "string", required: false, desc: "RFC3339 lower bound." },
            Param { name: "to", loc: Q, ty: "string", required: false, desc: "RFC3339 upper bound." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size." },
            Param { name: "cursor", loc: Q, ty: "string", required: false, desc: "Pagination cursor." },
        ],
    },
    ToolDef {
        name: "audit.checkpoint_get",
        desc: "Latest audit chain checkpoint.",
        method: "GET", path: "/audit/checkpoint", capability: Some("audit.read"), params: &[],
    },
    ToolDef {
        name: "audit.checkpoint_create",
        desc: "Create an audit chain checkpoint.",
        method: "POST", path: "/audit/checkpoint", capability: Some("security.manage"), params: &[],
    },
    ToolDef {
        name: "audit.verify",
        desc: "Verify audit chain integrity over a range.",
        method: "POST", path: "/audit/verify", capability: Some("security.manage"),
        params: &[
            Param { name: "from", loc: B, ty: "string", required: false, desc: "Range start." },
            Param { name: "to", loc: B, ty: "string", required: false, desc: "Range end." },
        ],
    },
    ToolDef {
        name: "audit.retention_get",
        desc: "Current audit retention policy.",
        method: "GET", path: "/audit/retention", capability: Some("audit.read"), params: &[],
    },
    ToolDef {
        name: "audit.retention_set",
        desc: "Update the audit retention policy.",
        method: "PUT", path: "/audit/retention", capability: Some("security.manage"),
        params: &[
            Param { name: "retention_days", loc: B, ty: "integer", required: true, desc: "1..36500 days." },
            Param { name: "legal_hold", loc: B, ty: "boolean", required: false, desc: "Freeze deletions." },
            Param { name: "archive_before_delete", loc: B, ty: "boolean", required: false, desc: "Archive first." },
            Param { name: "archive_url", loc: B, ty: "string", required: false, desc: "Archive location." },
        ],
    },

    // --- Permission groups -----------------------------------------------------------------
    ToolDef {
        name: "permissions.groups",
        desc: "List permission/role groups with rules and memberships.",
        method: "GET", path: "/permissions/groups", capability: Some("admin.pages.access"), params: &[],
    },
    ToolDef {
        name: "permissions.group_create",
        desc: "Create a role group (class data|capability, optional parent, rules, members).",
        method: "POST", path: "/permissions/groups", capability: Some("security.manage"),
        params: &[
            Param { name: "name", loc: B, ty: "string", required: true, desc: "Group alias." },
            Param { name: "class", loc: B, ty: "string", required: true, desc: "data|capability." },
            Param { name: "parent_id", loc: B, ty: "string", required: false, desc: "Parent role id (inheritance)." },
            Param { name: "description", loc: B, ty: "string", required: false, desc: "Description." },
            Param { name: "rules", loc: B, ty: "array", required: false, desc: "Policy rules." },
            Param { name: "principals", loc: B, ty: "array", required: false, desc: "Member logins/principal ids." },
        ],
    },
    ToolDef {
        name: "permissions.group_update",
        desc: "Update a role group (rules/members replaced as new validity windows).",
        method: "PUT", path: "/permissions/groups/{id}", capability: Some("security.manage"),
        params: &[
            ID,
            Param { name: "name", loc: B, ty: "string", required: true, desc: "Group alias." },
            Param { name: "parent_id", loc: B, ty: "string", required: false, desc: "Parent role id." },
            Param { name: "description", loc: B, ty: "string", required: false, desc: "Description." },
            Param { name: "rules", loc: B, ty: "array", required: false, desc: "Policy rules." },
            Param { name: "principals", loc: B, ty: "array", required: false, desc: "Member logins/ids." },
        ],
    },
    ToolDef {
        name: "permissions.group_delete",
        desc: "Delete a role group (soft-delete; fails when child roles exist).",
        method: "DELETE", path: "/permissions/groups/{id}", capability: Some("security.manage"),
        params: &[ID],
    },

    // --- User administration --------------------------------------------------------------
    ToolDef {
        name: "users.role_groups",
        desc: "Role groups available for user membership.",
        method: "GET", path: "/users/groups", capability: Some("user.read"), params: &[],
    },
    ToolDef {
        name: "users.list",
        desc: "List/search users (name, login, email, IP prefix, role, status).",
        method: "GET", path: "/users", capability: Some("user.read"),
        params: &[
            Param { name: "q", loc: Q, ty: "string", required: false, desc: "Free text over name/login/email." },
            Param { name: "group_id", loc: Q, ty: "string", required: false, desc: "Role group id filter." },
            Param { name: "inherited", loc: Q, ty: "string", required: false, desc: "1|true — include descendant roles." },
            Param { name: "ip", loc: Q, ty: "string", required: false, desc: "Last-login IP prefix." },
            Param { name: "status", loc: Q, ty: "string", required: false, desc: "active|disabled|locked." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size (default 200)." },
        ],
    },
    ToolDef {
        name: "users.get",
        desc: "User detail: roles (direct + inherited), sessions, status.",
        method: "GET", path: "/users/{id}", capability: Some("user.read"),
        params: &[ID],
    },
    ToolDef {
        name: "users.create",
        desc: "Create a user or service account (`is_service_account`), assign roles.",
        method: "POST", path: "/users", capability: Some("user.create"),
        params: &[
            Param { name: "login", loc: B, ty: "string", required: true, desc: "Globally unique login." },
            Param { name: "display_name", loc: B, ty: "string", required: true, desc: "Display name." },
            Param { name: "password", loc: B, ty: "string", required: false, desc: "Min 8 chars; optional for service accounts." },
            Param { name: "status", loc: B, ty: "string", required: false, desc: "active|disabled|locked." },
            Param { name: "groups", loc: B, ty: "array", required: false, desc: "Role ids or aliases." },
            Param { name: "is_service_account", loc: B, ty: "boolean", required: false, desc: "true for API/service accounts." },
            Param { name: "email", loc: B, ty: "string", required: false, desc: "Email." },
        ],
    },
    ToolDef {
        name: "users.update",
        desc: "Update a user: display_name/status/email/password/roles.",
        method: "PUT", path: "/users/{id}", capability: Some("user.edit"),
        params: &[
            ID,
            Param { name: "display_name", loc: B, ty: "string", required: false, desc: "New name." },
            Param { name: "status", loc: B, ty: "string", required: false, desc: "active|disabled|locked." },
            Param { name: "email", loc: B, ty: "string", required: false, desc: "Email." },
            Param { name: "password", loc: B, ty: "string", required: false, desc: "New password (min 8)." },
            Param { name: "groups", loc: B, ty: "array", required: false, desc: "Replacement role list." },
        ],
    },
    ToolDef {
        name: "users.delete",
        desc: "Disable a user (soft delete; ends sessions, revokes API tokens, closes memberships).",
        method: "DELETE", path: "/users/{id}", capability: Some("user.delete"),
        params: &[ID],
    },
    ToolDef {
        name: "users.activity",
        desc: "Per-user activity: created/changed/viewed entities + audit log.",
        method: "GET", path: "/users/{id}/activity", capability: Some("user.read"),
        params: &[
            ID,
            Param { name: "q", loc: Q, ty: "string", required: false, desc: "Free text over the log." },
            Param { name: "limit", loc: Q, ty: "integer", required: false, desc: "Page size." },
        ],
    },
    ToolDef {
        name: "users.tokens",
        desc: "List API tokens of a user (metadata only — never the secret).",
        method: "GET", path: "/users/{id}/tokens", capability: Some("user.read"),
        params: &[ID],
    },
    ToolDef {
        name: "users.token_create",
        desc: "Issue an API token for a user/service account. The plaintext token is returned once.",
        method: "POST", path: "/users/{id}/tokens", capability: Some("user.edit"),
        params: &[
            ID,
            Param { name: "name", loc: B, ty: "string", required: true, desc: "Token label." },
            Param { name: "expires_in_days", loc: B, ty: "integer", required: false, desc: "Optional expiry." },
        ],
    },
    ToolDef {
        name: "users.token_revoke",
        desc: "Revoke an API token.",
        method: "DELETE", path: "/users/{id}/tokens/{token_id}", capability: Some("user.edit"),
        params: &[
            ID,
            Param { name: "token_id", loc: P, ty: "string", required: true, desc: "Token id." },
        ],
    },
];

/// JSON Schema for a tool's `inputSchema` field.
pub fn input_schema(tool: &ToolDef) -> Value {
    let mut props = Map::new();
    let mut required = Vec::new();
    for p in tool.params {
        let loc = match p.loc {
            ParamLoc::Path => "path",
            ParamLoc::Query => "query",
            ParamLoc::Body => "body",
        };
        props.insert(
            p.name.to_string(),
            json!({ "type": p.ty, "description": p.desc, "x-location": loc }),
        );
        if p.required {
            required.push(json!(p.name));
        }
    }
    json!({
        "type": "object",
        "properties": Value::Object(props),
        "required": Value::Array(required),
        "additionalProperties": false,
    })
}

/// MCP `tools/list` entry for a tool.
pub fn tool_descriptor(tool: &ToolDef) -> Value {
    json!({
        "name": tool.name,
        "description": tool.desc,
        "inputSchema": input_schema(tool),
    })
}

/// Tools the given capability set permits. `None` capability = always
/// available. Umbrella grants (`users.manage` covering `user.*`) are expanded
/// here the same way the API guard treats them.
pub fn tools_for_capabilities(caps: &[String]) -> Vec<&'static ToolDef> {
    let has = |cap: &str| -> bool {
        if caps.iter().any(|c| c == cap) {
            return true;
        }
        // Umbrella: users.manage covers every user.* capability.
        cap.starts_with("user.") && caps.iter().any(|c| c == "users.manage")
    };
    TOOLS
        .iter()
        .filter(|t| t.capability.map_or(true, has))
        .collect()
}
