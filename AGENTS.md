# EngineR — project notes for agents

## Layout

- `apps/api` — Rocket HTTP API (`enginer-api`).
- `apps/worker` — background worker (`enginer-worker`): outbox drain loop.
- `crates/` — domain/application/contracts/observability/db-runtime/persistence/
  security-domain/security-pdp/security-api/schema-registry/identity/
  query-engine/test-support.
- `migrations/` — sqlx migrations for PostgreSQL schemas `core/meta/sec/ops/audit`.
- `web/` — typed API client (`@enginer/api-client`), TypeScript, Vitest.
- `nuxt/` — Nuxt 4 admin UI (SPA, Vue 3, Nuxt UI 4, i18n, Pinia).
- `legacy-prototype/` — old Rocket 0.5-rc prototype, **excluded** from workspace.
- `docs/` — normative specifications (28 documents). Source of truth for design.

## Phase status

- **Phase 0 (Foundation):** complete — workspace, CI, config, Rocket health/error,
  DbExecutor, PostgreSQL migrations, observability, local dev stack.
- **Phase 1 (Security/meta-kernel):** complete — tenant/identity/session, two
  group classes, PDP/PEP/RLS/audit; type/predicate/schema draft/publish.
- **Phase 2 (Entity graph):** complete — Entity aggregate + IdentityPolicy/Key,
  CreateEntity/RetireEntity/MergeEntity use cases, entity/statement/identity-key
  repositories with RLS, query engine (AST + security rewrite + SQL compiler),
  entity API routes (PEP-gated), worker outbox drain loop, migration
  `20260803000008_core_statement.sql` applied.
- **Phase 3 (Nuxt core/admin):** complete — typed API client (`web/`) +
  Nuxt 4 admin UI (`nuxt/`). `@enginer/api-client`: TypeScript types mirroring
  Rust contracts (envelope, error, entity, query, schema, security),
  `ApiClient` fetch wrapper with 401 refresh singleflight, If-Match/
  Idempotency-Key/CSRF headers, timeout, typed `ApiClientError`; endpoint
  methods for entities, schemas, security, health; 26 Vitest tests. Nuxt 4
  SPA: layout (header + collapsible sidebar + skip-link a11y), auth (login
  page, `useAuth` Pinia store, `/me/context` fetch, route middleware,
  `useCapability` UX guard), entity list (`/objects/[typeAlias]` with cursor
  pagination + search), entity card (`/entities/[publicId]` with 8 tabs +
  retire dialog), schema admin (`/admin/schema` with drafts/published tabs +
  validate/impact/publish/retire), i18n (8 locales: ru, en, ar (RTL), he (RTL),
  fr, de, es, zh — 33 error code keys each, RTL dir attribute switching),
  `useApi` composable bridging to API client; metadata-driven UI
  (EntityFormRenderer with sections/collapsible/unsaved-changes guard,
  FormField field registry for 22 field kinds, EntityAutocomplete with
  prefix search, MultiValueField with cardinality constraints,
  EntityListRenderer with filters/facets/sort/bulk-selection/row-actions,
  GraphView with SVG radial layout + accessible tree alternative +
  progressive node expansion + cycle markers); entity create page
  (`/objects/[typeAlias]/create` with EntityFormRenderer + duplicate
  suggestion dialog with exact/fuzzy match + identity mode handling +
  save draft to localStorage + unsaved changes guard), RelationsTab
  (sections by predicate, direction labels, effective/historical switch,
  add/link existing, qualifiers, validity, source, cursor pagination),
  DocumentsTab (upload dialog, document kinds with icons, qualifiers,
  validity, source, file metadata, download/preview/remove actions),
  HistoryTab (revision timeline with diff, compare two revisions, reason);
  117 Vitest tests (StatCard, FormField 11, MultiValueField 7,
  DuplicateSuggestionDialog 7, error codes, i18n locale parity across
  8 locales); Playwright E2E setup with persona matrix (admin,
  security_officer, manager, risk_analyst) + accessibility + i18n
  tests. Build passes. ChecksTab (check instances with status icons,
  evidence items, step-up auth, data-denied, 8 check kinds, 6
  statuses), RisksTab (risk factors with level badges, assessment
  summary, decision/rationale, approval pending, resolve action,
  new assessment dialog), RiskLevelBadge (4 levels with icons),
  TasksTab (workflow tasks with priority sort, blocking badge,
  overdue indicator, start/complete/cancel actions, 7 task kinds,
  6 statuses, create dialog), monitoring page (summary cards,
  monitors table with frequency/next-run/last-result, run-now
  action, 5 frequencies), reports page (summary cards, reports
  table with format/status/download, generate dialog with
  templates/formats, 5 formats, 5 statuses); SKR domain types
  (CheckInstance, EvidenceItem, RiskFactor, RiskAssessment,
  WorkflowTask, MonitorSchedule, MonitorResult, ReportInstance);
  121 Vitest tests. Build passes. SKR core schema bundle
  (`schema-packages/skr-core/`): 15 types (person, legal_entity, holding_view,
  address, contact_point, document, license, request, check, risk_factor,
  assessment, decision, task, contract, collateral), ~60 predicates (scalar
  + edge), 7 identity policies (INN/ОГРН/document/contact/request/check/
  contract), 9 view definitions (person_form, legal_entity_form, person_list,
  legal_entity_list, holding_tree, entity_graph, request_list, check_list,
  assessment_card), 4 workflows (request_processing, check_execution,
  risk_assessment, monitoring_cycle), Excel mapping profile for ЮЛ import
  (13 column mappings with entity creation + relations), ru/en translations
  for all types and predicates, bundle manifest with file hashes; 9 Rust
  integration tests validating structural integrity, referential consistency,
  duplicate IDs, edge/scalar invariants, identity predicate constraints.
  SKR bundle loader (`crates/schema-registry/src/bundle.rs`): parses JSON
  bundle files into `LoadedBundle` → `SchemaDraft` for validation/publish,
  validates referential integrity (domain/range, predicate slots), 10 Rust
  tests. Excel import pipeline (`crates/contracts/src/import.rs`):
  MappingProfile, ColumnMapping, CellParser (15 kinds: text/date/money/
  phone/email/integer/decimal/boolean/inn/ogrn/kpp/okved/opf/person_name),
  ConflictAction, EmptySemantics, ParsedRow/Cell, DryRunRowResult/Summary,
  PlannedStatement, RowOutcome; `parse_cell` normalizer for all 15 parsers
  (INN 10/12 digits, OGRN 13/15 digits, KPP 9 digits, phone E.164 RU,
  date DD.MM.YYYY→ISO, money currency strip, person name split), 23 Rust
  tests. Total: 45 Rust tests (19 schema-registry + 23 contracts + 3 existing).
  Connectors crate (`crates/connectors/`): Connector SDK trait
  (docs/14) — `Connector` trait with descriptor/validate_credentials/
  health_check/search/fetch/monitor; `ConnectorDescriptor` with
  `ConnectorKind` (internal/api/file_import/internet/manual),
  `ConnectorCapabilities` (search/fetch/monitor/file_import, supported
  target types/check kinds, max response bytes, raw payload retention),
  `LegalMetadata` (legal_basis/purpose/jurisdiction/license/retention/
  redistribution); `ConnectorInstance` + `ConnectorInstanceConfig`
  (tenant-specific: enabled ops, environment, base_url, proxy, secret
  references, rate_limit_rps, concurrency, timeout, retry, cache
  freshness, legal basis, permitted target types/fields, retention
  policy, trust rank); `TrustRank` (Untrusted=0/Low=25/Medium=50/
  High=75/Authoritative=100); `SourceObservation` + `ProvenanceMeta`
  + `MappingResult` + `MappedPredicate` + `IdentityResolution`
  (observation status, raw payload policy, provenance with content
  hash/trust rank/purpose, mapping with predicate ↔ source field);
  `RateLimiter` (token bucket with burst), `CircuitBreaker`
  (closed/open/half-open with failure threshold + recovery timeout),
  `backoff_delay` (exponential with cap); 14 Rust tests.
  Total workspace: 148 Rust tests, all passing.
  Risk engine crate (`crates/risk-engine/`): criteria DSL AST
  (docs/14 §8) — `Expr`/`ExprKind` with 18 node types (Literal, Compare,
  Range, Existence, Count, Age, Duration, StringExact, StringNormalizedSet,
  RelationExists, RelationPath, SourceFreshness, SourceTrust, SourceConflict,
  Arithmetic, Logical, Unary, DictionaryLookup, PreviousAssessmentChange);
  `Value` (Decimal/Integer/Text/Boolean/Date/DurationDays/PredicateRef/
  ObservationRef/Null); `ComparisonOp` (Eq/Ne/Lt/Le/Gt/Ge/Prefix/Contains/
  NormalizedEq); `BinaryOp` (Add/Sub/Mul/Div); `LogicalOp` (All/Any);
  `UnaryOp` (Not/Exists/Count); pure `evaluate()` function with
  `EvaluationContext` (predicate_values, observations, previous_factors);
  `Criterion` with severity/weight/explanation_key/evidence_selector/
  valid_interval/owner/tests; `CriterionSeverity` (Info/Low/Medium/High/
  Critical with weight contributions 0/10/25/50/100); `Factor` with
  dedup_key (target+criterion+evidence window), status (Active/Resolved/
  Suppressed/Overridden/Expired), suppression/override, evidence refs;
  `WeightedAggregator` with `ScoreConfig` (weighted average or max mode,
  per-factor penalty, max penalty cap); `RiskLevel` (Minimal/Low/Medium/
  High/Critical from score 0-100); `ScoreResult` with component scores;
  `Decision` (Approve/Reject/Escalate/Defer/ConditionalApprove) with
  system/manual constructors, requires_approval for High/Critical;
  `Assessment` aggregate (Draft/Approved/Superseded) with target/as_of,
  model/version, input watermark/fingerprint, factor refs, component
  scores, final score, uncertainty (freshness/coverage/conflicts/confidence),
  generated/approved actors, explanation; 31 Rust tests.
  Total workspace: 179 Rust tests, all passing.
  Workflow crate (`crates/workflow/`): declarative state machine
  (docs/13 §7) — `StateMachineDefinition` with states/transitions/guards/
  actions/timers/assignment/escalation/compensation/version; `StateMachine`
  running instance with `transition()` (validates from/to/trigger, records
  history, marks terminal), variables, terminal detection; `State` with
  initial/terminal flags; `StateTransition` with trigger/guard/actions/
  requires_approval; `TransitionGuard` with expression/required_capabilities/
  required_data_scope. Job state machine (docs/13 §4): `Job` with
  `JobState` (Queued→Leased→Running→WaitingExternal/WaitingUser→Succeeded/
  PartiallySucceeded/Failed/Cancelled), `JobStatus` (phase, progress
  numerator/denominator, heartbeat, attempts, deadline, lease, cursor,
  result/error, irreversible_step_started); methods: lease/start/heartbeat/
  update_progress/set_cursor/wait_external/wait_user/resume/succeed/
  partially_succeed/fail/cancel/mark_irreversible; cooperative cancellation
  with cannot_cancel_after. Task entity (docs/13 §8): `Task` with
  `TaskStatus` (Pending/Assigned/InProgress/Completed/Cancelled/Overdue/
  Escalated), `TaskPriority` (Low=1/Normal=5/High=10/Urgent=20),
  `AssignmentStrategy` (Direct/Group/Auto/SelfAssign); assign_to_user/
  assign_to_group/accept/complete/cancel/mark_overdue/escalate; business
  calendar version for due_date. Timers (docs/13 §6,§12): `Schedule` with
  `ScheduleConfig` (IANA timezone, calendar expression, misfire policy,
  concurrency policy, owner, quiet period), `MisfirePolicy` (RunOnce/Skip/
  RunNext), `EscalationTimer` with `TimerKind` (Sla/Escalation/Reminder/
  Monitor/Timeout), is_due/fire/cancel/link_to_task. Reactions (docs/13 §10,
  §14 §12): `ReactionConfig` with `ReactionTrigger`/`ReactionTriggerKind`
  (CriterionTriggered/AssessmentThreshold/AssessmentChanged/FactorDetected/
  FactorResolved/TaskOverdue/MonitorChangeDetected/EntityCreated/Changed/
  Retired/Schedule/Manual), `ActionKind` (CreateTask/ChangeCategory/
  StartCheck/StartReport/SendEmail/SendWebhook/SendMessage/NotifyExternal/
  RequestApproval/Escalate), cooldown/dedup_key/idempotency_key_template/
  timeout/retries/compensation/requires_approval; `Reaction` instance with
  `ReactionResult` (Success/Failed/Skipped/PendingApproval); 45 Rust tests.
  Total workspace: 224 Rust tests, all passing.
  Files crate (`crates/files/`): staged upload protocol (docs/15 §1, §3
  step 1) — `StagedUpload` with `UploadInit`/`UploadChunk`/`UploadComplete`/
  `UploadAbort`, `UploadStatus` (Initiated/Uploading/AllChunksReceived/
  Completed/Aborted/ScanFailed); `init()` validates size/chunk_size > 0,
  computes total_chunks; `receive_chunk()` idempotent (re-upload same hash
  OK, different hash errors), out-of-range check; `complete()` verifies
  checksum + size match; `abort()` clears chunks; `progress()` (0.0-1.0),
  `is_complete()`, `is_terminal()`. File scanning (docs/15 §2):
  `FileScanResult` with `FileScanStatus` (Pending/Scanning/Clean/
  AcceptedWithWarnings/Rejected/Failed), `FileSignature` (format,
  mime_type, matches_declared), `ScanFinding` (kind/severity/message/
  location), `ScanSeverity` (Info/Warning/Critical), `ScanConfig` (max_size,
  max_decompressed, check_formulas/macros/external_links, run_antivirus,
  verify_mime, allowed_formats); `mark_clean/mark_accepted_with_warnings/
  mark_rejected/mark_failed`, `critical_findings()`, `passed()`,
  `is_terminal()`; `FileScanner` trait (detect_format, scan). Object
  storage abstraction (docs/15 §1, §14): `ObjectMetadata` (content_hash,
  size, content_type, storage_class, classification, expires_at, legal_hold,
  encrypted, user_metadata), `StorageClass` (Hot/Cool/Archive),
  `StorageConfig`, `StorageUrl`; `with_ttl()`, `apply_legal_hold()/
  release_legal_hold()`, `is_expired()`, `add_metadata()`; `ObjectStorage`
  trait (store/retrieve/delete/exists/head); 23 Rust tests.
  Reporting crate (`crates/reporting/`): datasets (docs/15 §9) —
  `Dataset` with `DatasetColumn` (Text/Integer/Decimal/Money/Date/Datetime/
  Boolean/Uuid/Json, nullable, classification, format_pattern),
  `DatasetQuery` (query_type, target_type, filter, group_by, order_by,
  limit, parameters), `DatasetStatus` (Draft/Active/Materializing/Ready/
  Stale/Retired); `DatasetSnapshot` with pinned schema_revision_id/
  policy_revision_id, `mark_ready(row_count, content_hash, storage_url)`,
  `mark_stale()`, `is_ready()`. Report definitions (docs/15 §9):
  `ReportDefinition` with sections/tables/charts/summaries, default_locale/
  timezone, template_id/version, allowed_formats, page/row limits,
  confidentiality, watermark, allowed_audiences, requires_snapshot,
  schedule; `ExportFormat` (Xlsx/Csv/Ndjson/Json/Docx/Pdf) with extension/
  mime_type; `ReportAudience` (audience_type, audience_id, can_reexport);
  `is_format_allowed()`, `is_audience_allowed()`, `all_tables()`,
  `all_charts()`, `all_summaries()`. Templates (docs/15 §9):
  `TemplateDefinition` with `TemplateVersion` (version, format, locales,
  field_formats, current), `TemplateLocale` (locale, storage_url,
  content_hash), `FieldFormat` (field_name, pattern, masked, mask_pattern),
  `TemplateFormat` (Docx/Xlsx/Pdf/Html); `current_version()`,
  `current_locale()`, `is_dataset_allowed()`, `add_version()` (marks
  previous as non-current). Rendering (docs/15 §8): `RenderContext`
  (tenant, actor, locale, timezone, format, scheduled, snapshot_grant_id,
  watermark, encrypt, sign, classification), `RenderOutput` (output_id,
  definition/version, template_version, schema/policy revision, format,
  storage_url, content_hash, size, page_count, row_count, rendered_at,
  duration_ms, status, encrypted, signed, ttl_days), `RenderStatus`
  (Pending/Rendering/Completed/Failed); `mark_completed()`, `mark_failed()`,
  `is_terminal()`; `TemplateRenderer` trait; `escape_formula_injection()`
  (docs/15 §8 — escapes =, +, -, @ prefixes); 25 Rust tests.
  Total workspace: 272 Rust tests, all passing.
  Security crate (`crates/security/`): PDP/PEP (docs/08 §1, §4) —
  `LocalPdpClient` with `PolicySnapshot` (revision_id, tenant_id,
  groups_revision_id, capabilities, capability_groups, data_rules);
  `Principal` (principal_id, kind User/ServiceAccount/System, tenant_id,
  group_ids, auth_strength, purpose); `SessionInfo` (session_id, device_id,
  ip_address, user_agent, mfa_verified, step_up, risk_score);
  `DecisionContext` (action, resource_type/id, requested_fields/relations,
  high_risk); `Decision` with `DecisionKind` (Allow/Deny/BreakGlassRequired/
  StepUpRequired), constructors: allow/deny/break_glass_required/
  step_up_required; `PdpClient` trait (evaluate, policy_snapshot);
  `evaluate_internal()` implements 9-step evaluation (docs/08 §4): tenant
  check, capability check, step-up check, data rule gathering, deny-overrides,
  allowed set + obligations, data permission resolution. Capabilities
  (docs/08 §2): `Capability` (name, kind System/Data, description_key,
  requires_step_up, high_risk); `CapabilityGroup` (group_id, name, kind
  System/Data, capabilities, owner); `CapabilityGroupKind` (System/Data);
  `standard_system_capabilities()` — 21 capabilities (login.use_ui,
  entity.create/edit/retire/merge, query.advanced/aggregate, check.run/
  monitor.manage, report.run/export, bulk.import/export/mutate, schema.draft/
  publish, security.manage/simulate, connector.manage, audit.read,
  admin.pages.access); step-up for publish/export/security/break-glass;
  high_risk for merge/export/mutate/publish/security/admin. ABAC (docs/08
  §3): `Effect` (Allow/Deny, deny-overrides); `AttributeValue` (Tenant/
  Namespace/EntityType/EntityId/PredicateField/RelationPredicate/
  RelationDirection/Classification/Tag/Owner/RowCondition/PeriodStart/
  PeriodEnd/Purpose); `ResourceSelector` (tenant_id, entity_type, entity_id,
  fields, relations, classifications, tags, row_condition); `DataRule`
  (rule_id, effect, subject, action, resource, condition, fields, relations,
  mask, obligations, priority, valid_from/to, policy_revision_id, reason);
  `is_valid_at()`, `is_deny()`, `matches()`; `DataPermission` (allowed/
  denied_types, allowed/denied_fields per type, allowed/denied_relations,
  row_condition, allowed_classifications, obligations, mask);
  `is_type_allowed()`, `is_field_allowed()`, `is_relation_allowed()`.
  Classification (docs/08 §7): `ClassificationLevel` (Public/Internal/
  Restricted/Confidential/Secret with rank 0-4); `is_allowed_with()`,
  `from_str()`, `as_str()`; `FieldPolicyKind` (Allow/Omit/NullMask/
  PartialMask/Tokenize/AggregateOnly); `MaskStrategy` (null_mask/
  partial_mask/tokenize with apply()); `FieldPolicy` (allow/omit/null_mask/
  partial_mask constructors with apply()). Audit (docs/08 §11):
  `AuditEntry` (entry_id, sequence, previous_hash, hash, timestamp, tenant,
  actor/effective_actor, kind, action, resource_type/id, query_fingerprint,
  decision, reason_ids, policy_revision_id, request/correlation_id,
  row_count, bytes, client/service, metadata); `AuditKind` (Authentication/
  Decision/DeniedRequest/PolicyChange/Export/BlobRead/BulkAction/BreakGlass/
  Impersonation); builders: with_effective_actor/with_resource/with_decision/
  with_policy_revision/with_request/with_metrics/with_client/add_metadata;
  `compute_hash()` (FNV-1a-like); `TamperChain` (sequence, last_hash,
  checkpoints, total_entries) with append/checkpoint/verify (tamper
  detection); `AuditQuery` (tenant_id, kind, actor, action, resource,
  decision, from/to, limit, cursor); `AuditRetention` (retention_days,
  legal_hold, archive_before_delete, archive_url) with is_expired();
  51 Rust tests.
  Total workspace: 323 Rust tests, all passing.
  API endpoints Phase 5 (`apps/api/src/`): 4 new route modules wired into
  `rocket_build()` alongside existing entity routes. Schema admin routes
  (`schema_routes.rs`, docs/06 §5, docs/03 §7): `GET /api/v1/schema/drafts`
  (list), `GET /api/v1/schema/drafts/<id>` (get), `POST /api/v1/schema/drafts`
  (create), `POST /api/v1/schema/drafts/<id>/validate` (validate),
  `POST /api/v1/schema/drafts/<id>/impact` (impact analysis),
  `POST /api/v1/schema/drafts/<id>/publish` (publish, step-up required),
  `POST /api/v1/schema/published/<id>/retire` (retire); PEP gates:
  `schema.draft` for draft/validate/impact, `schema.publish` for
  publish/retire. Connector CRUD routes (`connector_routes.rs`, docs/06 §7,
  docs/14 §4): `GET /api/v1/connectors` (list), `GET /api/v1/connectors/<id>`
  (get), `POST /api/v1/connectors` (create), `PATCH /api/v1/connectors/<id>`
  (update), `POST /api/v1/connectors/<id>/enable` (enable),
  `POST /api/v1/connectors/<id>/disable` (disable),
  `POST /api/v1/connectors/<id>/health` (health check),
  `DELETE /api/v1/connectors/<id>` (delete); PEP gate: `connector.manage`.
  Check + Assessment + Workflow + Task routes (`check_routes.rs`, docs/06 §8,
  docs/14 §8-10, docs/13 §7-8): `POST /api/v1/checks` (request check),
  `GET /api/v1/checks/<id>` (get status), `GET /api/v1/checks/<id>/evidence`
  (get evidence), `GET /api/v1/entities/<id>/assessment` (get latest),
  `POST /api/v1/entities/<id>/assessment` (trigger),
  `GET /api/v1/entities/<id>/factors` (get factors),
  `POST /api/v1/workflows` (start), `GET /api/v1/workflows/<id>` (status),
  `POST /api/v1/workflows/<id>/transition` (transition),
  `GET /api/v1/workflows/<id>/tasks` (list tasks),
  `POST /api/v1/tasks/<id>/complete` (complete),
  `POST /api/v1/tasks/<id>/cancel` (cancel); PEP gates: `check.run` for
  checks/assessments, `entity.create` for workflow start, `entity.edit` for
  transition/tasks. File routes (`file_routes.rs`, docs/15 §1-3):
  `POST /api/v1/files/upload/init` (init staged upload),
  `PUT /api/v1/files/upload/<id>/chunks/<index>` (upload chunk),
  `POST /api/v1/files/upload/<id>/complete` (complete),
  `POST /api/v1/files/upload/<id>/abort` (abort),
  `GET /api/v1/files/upload/<id>` (status),
  `GET /api/v1/files/<id>` (download), `DELETE /api/v1/files/<id>` (delete);
  PEP gates: `bulk.import` for upload/delete (high_risk + step_up),
  `bulk.export` for download (high_risk + step_up). All routes return
  501 Not Implemented (persistence wired in full deployment); route
  structure, PEP gates and DTO mapping are the deliverable. Total: 35 new
  API endpoints across 4 modules.
  Integration crate (`crates/integration/`): end-to-end check flow
  orchestrator (docs/13 §8, docs/14 §4-12) — `CheckFlowOrchestrator`
  coordinates the full pipeline via 7 ports (CriterionPort, ConnectorPort,
  FactorPort, AssessmentPort, DecisionPort, WorkflowPort, TaskPort);
  `CheckFlowContext` tracks state through 12 states (Initiated→Fetching→
  Mapping→Evaluating→FactorDetection→Scoring→Assessing→Deciding→
  WorkflowStart→TaskCreation→Completed/Failed); `CheckFlowInput` (tenant,
  target_entity, target_type, check_kind, connector_instance, actor, force);
  `CheckFlowResult` (check_id, state, observation_count, criteria_evaluated,
  factors_detected, final_score, risk_level, assessment_id, decision,
  workflow_instance_id, tasks_created, warnings, completed_at); 8-step
  execute(): fetch observations → map → load criteria → evaluate + detect
  factors → aggregate score → create assessment → make decision → start
  workflow + create tasks (if escalation); 11 Rust tests with mock ports
  (completes_with_no_factors, detects_factors_and_scores,
  creates_workflow_on_escalation, state_transitions,
  failed_state_is_terminal, handles_empty_observations,
  handles_no_criteria, result_has_correct_target, decision_has_target_entity,
  completed_at_is_set, check_id_is_uuid).
  Total workspace: 334 Rust tests, all passing.
  Audit persistence crate (`crates/audit-persistence/`): tamper-evident
  chain + repository (docs/08 §11) — `ChainHasher` with BLAKE2b-256
  (compute_hash, verify_hash, verify_chain); `ChainState` (last_sequence,
  last_hash, from_checkpoint); `ChainEntry` for verification; `AuditRecord`
  (audit_id, sequence, previous_hash, entry_hash, tenant_id, occurred_at,
  actor, effective_actor, kind, action, resource_kind/id, effect, reason_ids,
  policy_revision, request/correlation_id, query_fingerprint, row_count,
  bytes, client, service, metadata); `AuditQuery` with builders (for_tenant,
  with_kind/actor/action/effect/limit/cursor/time_range); `RetentionPolicyRecord`
  (retention_days, legal_hold, archive_before_delete, archive_url);
  `ChainCheckpoint` (checkpoint_id, tenant_id, sequence, entry_hash);
  `AuditRepository` trait (insert, query, chain_state, checkpoint,
  latest_checkpoint, verify_chain, retention_policy, set_retention_policy);
  `InMemoryAuditRepository` (full impl for testing); `PgAuditRepository`
  (PostgreSQL with RLS, async runtime wired in production); migration
  `20260803000009_audit_chain.sql` (extends audit.event with sequence,
  previous_hash, entry_hash, kind, effective_actor, query_fingerprint,
  reason_ids, row_count, bytes, client, service, metadata; audit.chain_checkpoint
  table with RLS; audit.retention_policy table with RLS; retention role
  grants); 22 Rust tests (chain_state_default, next_sequence, compute_hash
  deterministic/different_inputs, verify_hash valid/invalid, verify_chain
  valid/tampered/broken_link/sequence_gap/empty, in_memory insert/query/
  chain_sequential/verify_chain/query_filters/query_cursor/checkpoint/
  retention_policy/default/tenant_isolation, audit_record_from_entry,
  audit_query_builders).
  Total workspace: 356 Rust tests, all passing.
  Phase 5 TS client + UI: `@enginer/api-client` extended with 4 new
  endpoint groups + 4 new type modules; Nuxt 4 admin UI extended with
  3 new pages. API client: `ConnectorEndpoints` (list/get/create/update/
  enable/disable/healthCheck/delete, 8 methods); `CheckEndpoints`
  (requestCheck/getCheck/getCheckEvidence/getAssessment/triggerAssessment/
  getFactors/startWorkflow/getWorkflow/transitionWorkflow/listWorkflowTasks/
  completeTask/cancelTask, 12 methods); `FileEndpoints` (uploadInit/
  uploadChunk/uploadComplete/uploadAbort/uploadStatus/getFile/deleteFile,
  7 methods); `AuditEndpoints` (query/latestCheckpoint/createCheckpoint/
  verifyChain/getRetentionPolicy/setRetentionPolicy, 6 methods); 33 new
  endpoint methods total. Types: `connector.ts` (ConnectorInstance,
  CreateConnectorBody, UpdateConnectorBody, ConnectorHealthCheck,
  TrustRank const); `check.ts` (CheckRequestBody, CheckResponse,
  EvidenceResponse, AssessmentResponse, FactorSummary, RiskLevel const,
  CheckKind/CheckStatus/TaskStatus/TaskPriority/DecisionKind consts,
  StartWorkflowBody, WorkflowResponse, TransitionBody, TaskResponse,
  CompleteTaskBody, CancelTaskBody, TriggerAssessmentBody); `file.ts`
  (UploadInitBody/Response, UploadCompleteBody, UploadAbortBody,
  UploadStatusResponse, FileMetadataResponse, UploadStatus/StorageClass
  consts); `audit.ts` (AuditRecord, AuditKind const, AuditQuery,
  ChainCheckpoint, RetentionPolicy, ChainVerificationResult). ApiClient
  extended with `putRaw()` for binary uploads. `EnginerClient` interface
  now has 8 endpoint groups (entities, schemas, security, health,
  connectors, checks, files, audit). 42 new Vitest tests in
  `web/tests/phase5-endpoints.test.ts` (8 connector, 12 check, 6 file,
  6 audit, 10 type constants). Web: 68 tests total (4 files), all
  passing. Nuxt UI: 3 new pages — `/checks/index.vue` (check list with
  filters by kind/status/entity, summary cards, new check dialog, status
  badges, risk level badges, 6 check kinds, 5 statuses); `/audit/index.vue`
  (audit log with filters by kind/actor/action/time range, chain
  verification banner, create checkpoint button, latest checkpoint card,
  cursor pagination, 9 audit kinds, effect badges, hash display);
  `/integrations/index.vue` (connector list with summary cards, trust
  rank visual bar, enable/disable toggle, health check button, new
  connector dialog with type/environment/rate_limit/trust_rank, 5 trust
  levels). i18n: 3 new sections (checks, audit, integrations) added to
  all 8 locales (ru, en, ar, de, es, fr, he, zh) with full translations.
  Nuxt: 121 tests total (7 files), all passing. Build passes.
  Audit API + wiring: `audit_routes.rs` (6 endpoints: GET /audit with
  10 query params for filters+cursor, GET /audit/checkpoint, POST
  /audit/checkpoint, GET /audit/verify, GET /audit/retention, PUT
  /audit/retention); `audit.view` capability for read, `audit.admin`
  for checkpoint/retention; DTOs: AuditRecordDto (23 fields), ChainCheckpointDto,
  ChainVerificationDto, RetentionPolicyDto; `audit_routes::routes()` mounted
  at `/api/v1`. `AuditPort` trait added to integration crate (record()
  method, non-fatal errors). `CheckFlowOrchestrator` now has optional
  `audit_port: Option<Arc<dyn AuditPort>>` with `with_audit_port()` builder;
  3 audit events emitted per flow: `check.run` (initiated, with check_kind/
  target_entity/connector), `check.decision` (with effect: allow/deny/
  escalate/defer + assessment_id + score + risk_level), `check.completed`
  (with state/observation_count/criteria_evaluated/factors_detected/final_score/
  tasks_created/workflow_instance_id). 3 new integration tests
  (records_audit_events, audit_decision_recorded_with_effect,
  without_audit_port_still_works). `ChainVerificationResult` struct added
  to audit-persistence crate. Total workspace: 355 Rust tests, all passing.
  Audit UI wiring: audit page enhanced with expandable detail rows (14
  detail fields), retention policy card with edit dialog, toast
  notifications for all async ops, reset filters button; checks page
  enhanced with toast error handling; integrations page enhanced with
  toast notifications (enabled_ok/disabled_ok/healthy_ok/healthy_fail/
  created_ok). i18n: 8 new common keys + 16 audit.detail keys + 7
  audit.retention keys + 1 checks.new_success + 5 integrations toast keys
  added to ru.ts and en.ts; common+audit.detail+audit.retention also
  added to all 6 other locales. Nuxt: 121 tests, all passing. Build passes.
  E2E tests (Playwright): 4 new spec files in `nuxt/e2e/` —
  `audit.spec.ts` (16 tests: page header with verify/checkpoint buttons,
  filter section with kind/actor/action/from/to, 3-row table rendering,
  kind badges, effect badges (allow/escalate/deny), expandable detail row
  on click, metadata JSON display, reason IDs badges, verify chain
  success banner, create checkpoint API call, latest checkpoint card,
  retention policy card with edit button, retention edit dialog save,
  reset filters, keyboard navigation, cursor pointer on rows, truncated
  hash display), `checks.spec.ts` (10 tests: page header with new check
  button, summary cards, kind/status filter selects, new check dialog
  with entity/kind/connector fields, submit button, check kind options,
  status badge colors, keyboard navigation, filter reset),
  `integrations.spec.ts` (13 tests: page header with new connector
  button, summary cards, 3-row connector table, type/environment/enabled
  display, trust rank visual bar (75/50/100), enable/disable toggle,
  health check button, new connector dialog with type/environment/
  rate_limit/trust_rank fields, 5 trust levels in select, health check
  API call, enable toggle API call, keyboard navigation, disabled
  badge), `accessibility.spec.ts` (20 tests: main landmark + h1 for
  /audit /checks /integrations, exactly one h1 per page, buttons have
  accessible names, table has thead/tbody, keyboard navigable, filter
  inputs have labels, expandable rows cursor pointer, no console
  errors). All E2E tests use mocked API routes (me/context, audit,
  checks, connectors). Total: 69 Playwright E2E tests (16+10+13+20+10
  existing). Vitest: 121 tests, all passing. Build passes.

## Verification commands

```bash
# Lint / format
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Type-check only (fast, no DB needed)
cargo check --workspace --all-targets

# Unit + invariant tests (no DB needed — the db-runtime bridge test is #[ignore])
cargo test --workspace

# Web (typed API client) — typecheck + tests
cd web && npm run typecheck && npm test

# Nuxt (admin UI) — build + tests
cd nuxt && npm run build && npm test

# DB-dependent tests / migrations
docker compose up -d postgres
# Apply migrations as the admin role (Windows: pipe via Get-Content):
#   Get-Content migrations/<file>.sql -Raw | docker exec -i enginer-postgres \
#     psql -U enginer_admin -d enginer
# Migrations in order:
#   20260803000001_baseline.sql
#   20260803000002_core_entity.sql
#   20260803000003_meta_registry.sql
#   20260803000004_sec_security.sql
#   20260803000005_ops_outbox.sql
#   20260803000006_audit_event.sql
#   20260803000007_integration.sql
#   20260803000008_core_statement.sql   (Phase 2: statement/identity_key/identity_policy)

# Run the API locally
ENGINER_CONFIG=config/enginer.local.toml cargo run -p enginer-api
# Health: curl http://localhost:8000/health/live
# Entity query: POST http://localhost:8000/api/v1/entities/query
```

## Test inventory

- `enginer-domain`: ids roundtrip, scalar canonical bytes, fingerprint stability,
  valid-period invariants, schema/statement/time domain checks, entity aggregate
  (retire/restore/merge idempotency, self-merge rejection, revision monotonicity,
  retire→restore→retire cycle, merge-into-different-canonical), identity key
  separator/order invariants.
- `enginer-identity`: normalizer idempotency (text/phone/email), Unicode
  preservation, casefold search key.
- `enginer-schema-registry`: draft→publish immutability, duplicate detection,
  dangling range rejection, content-hash freeze.
- `enginer-security-pdp`: default-deny, deny-overrides, data-group does not
  grant capability.
- `enginer-application`: CreateEntity (happy path, dedup returns existing,
  identity key insertion, no-statements, outbox payload), RetireEntity (happy
  path, not-found, idempotent no-double-bump), MergeEntity (happy path,
  self-merge rejection, idempotent repeat, canonical/loser not-found). Uses
  in-memory mock repositories (`test_mocks.rs`).
- `enginer-query-engine`: AST JSON roundtrip, syntactic limits, security
  rewriter (page clamp, root-type deny, hidden-field drop from select/sort,
  mandatory filter injection), SQL compiler (no user literal in SQL text,
  predicate comparison via EXISTS, bind placeholders, AND/OR/NOT, IN, Between,
  IsNull/IsNotNull, Relation exists outgoing/incoming, Contains/Prefix,
  empty select with tie-breaker, DESC sort with ASC tie-breaker, full
  pipeline rewrite→compile, IN empty values error, Between wrong arity error).
- `enginer-test-support` (cross-crate invariants): all of the above end-to-end
  through `LocalSecurityClient`, plus decision-binding/expiry and
  query.aggregate capability gating.
- `enginer-db-runtime`: `DbError` classification (the live-bridge test is
  `#[ignore]` because it needs a non-listening port that refuses quickly).

## Conventions

- IDs are newtypes in `enginer-domain::ids`; never pass identifiers as interchangeable `String`.
- SQL is built with sqlx `QueryBuilder` + bind parameters only inside `enginer-persistence`.
  No `format!`-built SQL with user values (legacy prototype did this — not reused).
- The query engine compiler produces parameterized SQL with `$N` placeholders;
  user literals never appear in the SQL text (docs/07 §6, §14).
- Application ports return `BoxFuture` so the traits are object-safe (`dyn Trait`);
  use cases hold `Arc<dyn Repository>`.
- Security is default-deny; every controller/use case passes through PEP.
- RLS context (`set_config`) is set per-transaction and reset before return.
- `unsafe` is forbidden workspace-wide.
- Decimal as string over the wire; `f64` forbidden for money/shares/scores.

## Deviation from docs

- `sqlx` is 0.7.4 (not 0.5.13): 0.5's transitive deps no longer compile on Rust 1.95+.
  0.7.4 still provides `runtime-async-std-native-tls` and the same async-std PgPool
  ownership model. The `DbExecutor` port isolates the runtime, so this is a
  single-point ADR deviation. See root `Cargo.toml` header.
- Rocket route path params use `&str` (not `uuid::Uuid`) because Rocket 0.5's
  `FromParam` is not implemented for `uuid::Uuid`; the handler parses the string.
