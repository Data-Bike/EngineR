<template>
  <div class="mx-auto max-w-7xl space-y-6 p-4 sm:p-6" aria-labelledby="schema-page-title">
    <header class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <div class="mb-2 flex items-center gap-2 text-sm text-[var(--ui-text-muted)]">
          <UIcon name="i-lucide-database-zap" aria-hidden="true" />
          <span>{{ t('schema.registry') }}</span>
        </div>
        <h1 id="schema-page-title" class="text-2xl font-bold">{{ t('schema.title') }}</h1>
        <p class="mt-1 max-w-3xl text-[var(--ui-text-muted)]">{{ t('schema.description') }}</p>
      </div>
      <div class="flex flex-wrap gap-2">
        <UButton
          variant="outline"
          icon="i-lucide-refresh-cw"
          :loading="initialLoading"
          @click="reloadAll"
        >
          {{ t('schema.refresh') }}
        </UButton>
        <UButton icon="i-lucide-plus" :disabled="isBusy" @click="openCreateDialog">
          {{ t('schema.create_draft') }}
        </UButton>
      </div>
    </header>

    <UAlert
      v-if="loadError"
      color="error"
      variant="soft"
      icon="i-lucide-circle-alert"
      :title="t('schema.load_failed')"
      :description="loadError"
    >
      <template #actions>
        <UButton size="sm" color="error" variant="outline" @click="reloadAll">
          {{ t('common.retry') }}
        </UButton>
      </template>
    </UAlert>

    <section class="grid grid-cols-2 gap-3 lg:grid-cols-4" :aria-label="t('schema.summary')">
      <UCard>
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.drafts') }}</p>
            <p class="mt-1 text-2xl font-semibold">{{ drafts.length }}</p>
          </div>
          <UIcon name="i-lucide-file-pen-line" class="size-6 text-warning" aria-hidden="true" />
        </div>
      </UCard>
      <UCard>
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.ready_to_publish') }}</p>
            <p class="mt-1 text-2xl font-semibold">{{ readyDrafts }}</p>
          </div>
          <UIcon name="i-lucide-badge-check" class="size-6 text-success" aria-hidden="true" />
        </div>
      </UCard>
      <UCard>
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.current_revision') }}</p>
            <p class="mt-1 truncate text-lg font-semibold">{{ currentRevision?.semantic_version ?? '—' }}</p>
          </div>
          <UIcon name="i-lucide-git-commit-horizontal" class="size-6 text-primary" aria-hidden="true" />
        </div>
      </UCard>
      <UCard>
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.breaking_changes') }}</p>
            <p class="mt-1 text-2xl font-semibold">{{ breakingDrafts }}</p>
          </div>
          <UIcon name="i-lucide-triangle-alert" class="size-6 text-error" aria-hidden="true" />
        </div>
      </UCard>
    </section>

    <UCard>
      <UTabs v-model="activeTab" :items="tabs" class="w-full">
        <template #content="{ item }">
          <section v-if="item.value === 'drafts'" class="space-y-4 pt-4">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <UInput
                v-model="search"
                icon="i-lucide-search"
                :placeholder="t('schema.search_placeholder')"
                class="w-full sm:max-w-sm"
                :aria-label="t('schema.search_placeholder')"
              />
              <USelect
                v-model="draftStateFilter"
                :items="draftStateOptions"
                class="w-full sm:w-56"
                :aria-label="t('schema.filter_state')"
              />
            </div>

            <div v-if="initialLoading" class="space-y-3" aria-live="polite">
              <USkeleton v-for="index in 3" :key="index" class="h-36 w-full" />
            </div>
            <div
              v-else-if="filteredDrafts.length === 0"
              class="rounded-lg border border-dashed border-[var(--ui-border)] px-6 py-12 text-center"
            >
              <UIcon name="i-lucide-file-search" class="mx-auto mb-3 size-10 text-[var(--ui-text-muted)]" aria-hidden="true" />
              <h2 class="font-medium">{{ search || draftStateFilter !== 'all' ? t('schema.no_matches') : t('schema.no_drafts') }}</h2>
              <p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ t('schema.no_drafts_hint') }}</p>
              <UButton v-if="!search && draftStateFilter === 'all'" class="mt-4" icon="i-lucide-plus" @click="openCreateDialog">
                {{ t('schema.create_draft') }}
              </UButton>
            </div>

            <article
              v-for="draft in filteredDrafts"
              v-else
              :key="draft.revision_id"
              class="rounded-lg border border-[var(--ui-border)] bg-[var(--ui-bg)] p-4 transition-colors hover:border-[var(--ui-border-accented)]"
            >
              <div class="flex flex-wrap items-start justify-between gap-4">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <h2 class="font-semibold">{{ draft.display_name }}</h2>
                    <UBadge :color="statusColor(draft.status)" variant="soft" size="sm">
                      {{ statusLabel(draft.status) }}
                    </UBadge>
                    <UBadge v-if="draft.impact?.breaking" color="error" variant="soft" size="sm">
                      {{ t('schema.breaking') }}
                    </UBadge>
                  </div>
                  <p v-if="draft.description" class="mt-1 line-clamp-2 text-sm text-[var(--ui-text-muted)]">
                    {{ draft.description }}
                  </p>
                  <div class="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-xs text-[var(--ui-text-muted)]">
                    <span>v{{ draft.semantic_version }}</span>
                    <span>{{ t('schema.revision') }} #{{ draft.revision_id }}</span>
                    <span>{{ t('schema.types_count', { count: draft.type_count }) }}</span>
                    <span>{{ t('schema.predicates_count', { count: draft.predicate_count }) }}</span>
                    <span>{{ t('schema.updated') }} {{ formatDate(draft.updated_at) }}</span>
                  </div>
                </div>

                <div class="flex flex-wrap justify-end gap-2">
                  <UButton size="sm" variant="outline" icon="i-lucide-eye" @click="openDetail(draft, 'draft')">
                    {{ t('common.view') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-pencil"
                    :disabled="isBusy"
                    @click="openEditDialog(draft)"
                  >
                    {{ t('common.edit') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-check-circle"
                    :loading="busyAction === `validate:${draft.revision_id}`"
                    :disabled="isBusy || draft.status !== 'draft'"
                    @click="runValidation(draft)"
                  >
                    {{ t('schema.validate') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="outline"
                    icon="i-lucide-git-compare"
                    :loading="busyAction === `impact:${draft.revision_id}`"
                    :disabled="isBusy || !draft.validation?.valid || draft.status !== 'validating'"
                    @click="runImpact(draft)"
                  >
                    {{ t('schema.impact') }}
                  </UButton>
                  <UButton
                    size="sm"
                    icon="i-lucide-rocket"
                    :disabled="isBusy || !canPublishSchema || draft.status !== 'impact_ready' || Boolean(draft.impact?.blockers.length)"
                    @click="openPublishDialog(draft)"
                  >
                    {{ t('schema.publish') }}
                  </UButton>
                  <UButton
                    size="sm"
                    variant="ghost"
                    color="error"
                    icon="i-lucide-trash-2"
                    :aria-label="t('schema.delete_draft')"
                    :disabled="isBusy"
                    @click="openDeleteDialog(draft)"
                  />
                </div>
              </div>

              <div class="mt-4 grid gap-2 sm:grid-cols-2">
                <div class="flex items-center gap-2 rounded-md bg-[var(--ui-bg-muted)] px-3 py-2 text-sm">
                  <UIcon
                    :name="draft.validation?.valid ? 'i-lucide-circle-check' : 'i-lucide-circle-dashed'"
                    :class="draft.validation?.valid ? 'text-success' : 'text-[var(--ui-text-muted)]'"
                    aria-hidden="true"
                  />
                  <span>{{ draft.validation?.valid ? t('schema.validation_passed') : t('schema.validation_pending') }}</span>
                </div>
                <div class="flex items-center gap-2 rounded-md bg-[var(--ui-bg-muted)] px-3 py-2 text-sm">
                  <UIcon
                    :name="draft.impact ? 'i-lucide-circle-check' : 'i-lucide-circle-dashed'"
                    :class="draft.impact ? (draft.impact.breaking ? 'text-error' : 'text-success') : 'text-[var(--ui-text-muted)]'"
                    aria-hidden="true"
                  />
                  <span>{{ draft.impact ? compatibilityLabel(draft.impact.compatibility) : t('schema.impact_pending') }}</span>
                </div>
              </div>
            </article>
          </section>

          <section v-else class="space-y-4 pt-4">
            <div v-if="initialLoading" class="space-y-3" aria-live="polite">
              <USkeleton v-for="index in 2" :key="index" class="h-32 w-full" />
            </div>
            <div
              v-else-if="revisions.length === 0"
              class="rounded-lg border border-dashed border-[var(--ui-border)] px-6 py-12 text-center text-[var(--ui-text-muted)]"
            >
              {{ t('schema.no_revisions') }}
            </div>
            <article
              v-for="revision in revisions"
              v-else
              :key="revision.revision_id"
              class="rounded-lg border border-[var(--ui-border)] bg-[var(--ui-bg)] p-4"
            >
              <div class="flex flex-wrap items-start justify-between gap-4">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <h2 class="font-semibold">{{ revision.display_name }}</h2>
                    <UBadge :color="statusColor(revision.status)" variant="soft" size="sm">
                      {{ statusLabel(revision.status) }}
                    </UBadge>
                    <UBadge v-if="revision.status === 'published'" color="primary" variant="outline" size="sm">
                      {{ t('schema.current') }}
                    </UBadge>
                  </div>
                  <p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ revision.description }}</p>
                  <div class="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-xs text-[var(--ui-text-muted)]">
                    <span>v{{ revision.semantic_version }}</span>
                    <span>{{ t('schema.revision') }} #{{ revision.revision_id }}</span>
                    <span>{{ t('schema.types_count', { count: revision.type_count }) }}</span>
                    <span>{{ t('schema.predicates_count', { count: revision.predicate_count }) }}</span>
                    <span v-if="revision.published_at">{{ t('schema.published_at') }} {{ formatDate(revision.published_at) }}</span>
                  </div>
                  <div v-if="revision.content_hash" class="mt-2 flex items-center gap-2 text-xs text-[var(--ui-text-muted)]">
                    <UIcon name="i-lucide-fingerprint" aria-hidden="true" />
                    <code class="break-all">{{ revision.content_hash }}</code>
                  </div>
                </div>
                <div class="flex flex-wrap gap-2">
                  <UButton size="sm" variant="outline" icon="i-lucide-eye" @click="openDetail(revision, 'revision')">
                    {{ t('common.view') }}
                  </UButton>
                  <UButton
                    v-if="revision.status === 'published'"
                    size="sm"
                    variant="outline"
                    color="error"
                    icon="i-lucide-archive"
                    :disabled="!canPublishSchema"
                    @click="openRetireDialog(revision)"
                  >
                    {{ t('schema.retire') }}
                  </UButton>
                </div>
              </div>
            </article>
          </section>
        </template>
      </UTabs>
    </UCard>

    <UModal v-model:open="createDialogOpen">
      <template #content>
        <form class="space-y-4 p-6" @submit.prevent="createDraft">
          <div>
            <h2 class="text-lg font-semibold">{{ t('schema.create_draft') }}</h2>
            <p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ t('schema.create_hint') }}</p>
          </div>
          <UFormField :label="t('schema.draft_name')" required>
            <UInput v-model="createForm.name" autofocus class="w-full" :placeholder="t('schema.draft_name_placeholder')" />
          </UFormField>
          <UFormField :label="t('schema.draft_description')">
            <UTextarea v-model="createForm.description" class="w-full" :rows="3" />
          </UFormField>
          <div class="grid gap-4 sm:grid-cols-2">
            <UFormField :label="t('schema.semantic_version')" required>
              <UInput v-model="createForm.semantic_version" class="w-full" placeholder="1.1.0" />
            </UFormField>
            <UFormField :label="t('schema.base_revision')">
              <USelect v-model="createForm.base_revision" :items="baseRevisionOptions" class="w-full" />
            </UFormField>
          </div>
          <p v-if="createError" class="text-sm text-error" role="alert">{{ createError }}</p>
          <div class="flex justify-end gap-2">
            <UButton type="button" variant="outline" @click="createDialogOpen = false">{{ t('common.cancel') }}</UButton>
            <UButton type="submit" :loading="busyAction === 'create'" :disabled="!canCreateDraft">{{ t('common.save') }}</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal v-model:open="editDialogOpen" :ui="{ content: 'sm:max-w-4xl' }">
      <template #content>
        <form class="max-h-[85vh] space-y-5 overflow-y-auto p-6" @submit.prevent="saveDraft">
          <div>
            <h2 class="text-lg font-semibold">{{ t('schema.edit_draft') }}</h2>
            <p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ t('schema.edit_hint') }}</p>
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <UFormField :label="t('schema.draft_name')" required>
              <UInput v-model="editForm.display_name" class="w-full" />
            </UFormField>
            <UFormField :label="t('schema.semantic_version')" required>
              <UInput v-model="editForm.semantic_version" class="w-full" />
            </UFormField>
          </div>
          <UFormField :label="t('schema.draft_description')">
            <UTextarea v-model="editForm.description" class="w-full" :rows="2" />
          </UFormField>

          <div class="grid gap-4 lg:grid-cols-2">
            <UFormField :label="t('schema.types_json')" :hint="t('schema.json_hint')">
              <UTextarea v-model="editForm.typesJson" class="w-full font-mono text-xs" :rows="12" spellcheck="false" />
            </UFormField>
            <UFormField :label="t('schema.predicates_json')" :hint="t('schema.json_hint')">
              <UTextarea v-model="editForm.predicatesJson" class="w-full font-mono text-xs" :rows="12" spellcheck="false" />
            </UFormField>
          </div>

          <section class="space-y-3" :aria-label="t('schema.change_log')">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h3 class="font-medium">{{ t('schema.change_log') }}</h3>
                <p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.change_log_hint') }}</p>
              </div>
              <UButton type="button" size="sm" variant="outline" icon="i-lucide-plus" @click="addChange">
                {{ t('schema.add_change') }}
              </UButton>
            </div>
            <div
              v-for="(change, index) in editForm.changes"
              :key="`${change.kind}:${change.resource_kind}:${change.resource_id}:${index}`"
              class="space-y-3 rounded-lg border border-[var(--ui-border)] p-3"
            >
              <div class="grid gap-3 sm:grid-cols-3">
                <UFormField :label="t('schema.change_kind')">
                  <USelect v-model="change.kind" :items="changeKindOptions" class="w-full" />
                </UFormField>
                <UFormField :label="t('schema.resource_kind')">
                  <UInput v-model="change.resource_kind" class="w-full" placeholder="predicate" />
                </UFormField>
                <UFormField :label="t('schema.resource_id')">
                  <UInput v-model="change.resource_id" class="w-full" placeholder="okved" />
                </UFormField>
              </div>
              <UFormField :label="t('schema.change_summary')">
                <UInput v-model="change.summary" class="w-full" />
              </UFormField>
              <div class="flex items-center justify-between gap-3">
                <UCheckbox v-model="change.breaking" :label="t('schema.mark_breaking')" />
                <UButton
                  type="button"
                  size="sm"
                  variant="ghost"
                  color="error"
                  icon="i-lucide-trash-2"
                  :aria-label="t('schema.remove_change')"
                  @click="removeChange(index)"
                />
              </div>
            </div>
          </section>

          <p v-if="editError" class="text-sm text-error" role="alert">{{ editError }}</p>
          <div class="flex justify-end gap-2">
            <UButton type="button" variant="outline" @click="editDialogOpen = false">{{ t('common.cancel') }}</UButton>
            <UButton type="submit" :loading="busyAction === 'save'" :disabled="!canSaveDraft">{{ t('common.save') }}</UButton>
          </div>
        </form>
      </template>
    </UModal>

    <UModal v-model:open="detailDialogOpen" :ui="{ content: 'sm:max-w-4xl' }">
      <template #content>
        <div class="max-h-[85vh] space-y-5 overflow-y-auto p-6">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 class="text-lg font-semibold">{{ detailRecord?.display_name }}</h2>
              <p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ detailRecord?.description }}</p>
            </div>
            <UBadge v-if="detailRecord" :color="statusColor(detailRecord.status)" variant="soft">
              {{ statusLabel(detailRecord.status) }}
            </UBadge>
          </div>
          <div v-if="detailLoading" class="space-y-3"><USkeleton class="h-24 w-full" /><USkeleton class="h-48 w-full" /></div>
          <template v-else-if="detailRecord">
            <dl class="grid gap-3 rounded-lg bg-[var(--ui-bg-muted)] p-4 text-sm sm:grid-cols-3">
              <div><dt class="text-[var(--ui-text-muted)]">{{ t('schema.revision') }}</dt><dd class="font-medium">#{{ detailRecord.revision_id }}</dd></div>
              <div><dt class="text-[var(--ui-text-muted)]">{{ t('schema.semantic_version') }}</dt><dd class="font-medium">{{ detailRecord.semantic_version }}</dd></div>
              <div><dt class="text-[var(--ui-text-muted)]">{{ t('schema.base_revision') }}</dt><dd class="font-medium">{{ detailRecord.parent_revision_id ?? '—' }}</dd></div>
              <div><dt class="text-[var(--ui-text-muted)]">{{ t('schema.types') }}</dt><dd class="font-medium">{{ detailRecord.type_count }}</dd></div>
              <div><dt class="text-[var(--ui-text-muted)]">{{ t('schema.predicates') }}</dt><dd class="font-medium">{{ detailRecord.predicate_count }}</dd></div>
              <div><dt class="text-[var(--ui-text-muted)]">{{ t('schema.updated') }}</dt><dd class="font-medium">{{ formatDate(detailRecord.updated_at) }}</dd></div>
            </dl>
            <section v-if="detailRecord.changes.length" class="space-y-2">
              <h3 class="font-medium">{{ t('schema.change_log') }}</h3>
              <div v-for="(change, index) in detailRecord.changes" :key="`${change.resource_kind}:${change.resource_id}:${index}`" class="flex items-start gap-2 rounded-md border border-[var(--ui-border)] p-3 text-sm">
                <UBadge :color="change.breaking ? 'error' : 'neutral'" variant="soft" size="sm">{{ change.kind }}</UBadge>
                <div><p class="font-medium">{{ change.summary }}</p><p class="text-xs text-[var(--ui-text-muted)]">{{ change.resource_kind }} · {{ change.resource_id }}</p></div>
              </div>
            </section>
            <div class="grid gap-4 lg:grid-cols-2">
              <details class="rounded-lg border border-[var(--ui-border)] p-3">
                <summary class="cursor-pointer font-medium">{{ t('schema.types_json') }}</summary>
                <pre class="mt-3 max-h-80 overflow-auto whitespace-pre-wrap text-xs">{{ prettyJson(detailRecord.types) }}</pre>
              </details>
              <details class="rounded-lg border border-[var(--ui-border)] p-3">
                <summary class="cursor-pointer font-medium">{{ t('schema.predicates_json') }}</summary>
                <pre class="mt-3 max-h-80 overflow-auto whitespace-pre-wrap text-xs">{{ prettyJson(detailRecord.predicates) }}</pre>
              </details>
            </div>
          </template>
          <div class="flex justify-end"><UButton variant="outline" @click="detailDialogOpen = false">{{ t('common.close') }}</UButton></div>
        </div>
      </template>
    </UModal>

    <UModal v-model:open="validationDialogOpen">
      <template #content>
        <div class="space-y-4 p-6">
          <div class="flex items-center gap-3">
            <UIcon :name="validationReport?.valid ? 'i-lucide-circle-check' : 'i-lucide-circle-x'" :class="validationReport?.valid ? 'size-7 text-success' : 'size-7 text-error'" aria-hidden="true" />
            <div><h2 class="text-lg font-semibold">{{ t('schema.validation_result') }}</h2><p class="text-sm text-[var(--ui-text-muted)]">{{ validationReport?.valid ? t('schema.validation_passed') : t('schema.validation_failed') }}</p></div>
          </div>
          <div v-if="validationReport?.findings.length" class="max-h-96 space-y-2 overflow-y-auto">
            <div v-for="finding in validationReport.findings" :key="`${finding.code}:${finding.path}`" class="rounded-md border border-[var(--ui-border)] p-3 text-sm">
              <div class="flex items-center gap-2"><UBadge :color="finding.severity === 'error' ? 'error' : finding.severity === 'warning' ? 'warning' : 'info'" variant="soft" size="sm">{{ finding.severity }}</UBadge><code>{{ finding.code }}</code></div>
              <p class="mt-2">{{ finding.message }}</p><p v-if="finding.path" class="mt-1 text-xs text-[var(--ui-text-muted)]">{{ finding.path }}</p>
            </div>
          </div>
          <p v-else class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.no_findings') }}</p>
          <div class="flex justify-end"><UButton @click="validationDialogOpen = false">{{ t('common.close') }}</UButton></div>
        </div>
      </template>
    </UModal>

    <UModal v-model:open="impactDialogOpen">
      <template #content>
        <div class="space-y-4 p-6">
          <div><h2 class="text-lg font-semibold">{{ t('schema.impact_result') }}</h2><p class="text-sm text-[var(--ui-text-muted)]">{{ impactReport ? compatibilityLabel(impactReport.compatibility) : '' }}</p></div>
          <div v-if="impactReport" class="grid grid-cols-2 gap-3">
            <div class="rounded-lg bg-[var(--ui-bg-muted)] p-3"><p class="text-2xl font-semibold">{{ impactReport.affected_entities }}</p><p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.affected_entities') }}</p></div>
            <div class="rounded-lg bg-[var(--ui-bg-muted)] p-3"><p class="text-2xl font-semibold">{{ impactReport.affected_statements }}</p><p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.affected_statements') }}</p></div>
            <div class="rounded-lg bg-[var(--ui-bg-muted)] p-3"><p class="text-2xl font-semibold">{{ impactReport.details.new_fields }}</p><p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.new_fields') }}</p></div>
            <div class="rounded-lg bg-[var(--ui-bg-muted)] p-3"><p class="text-2xl font-semibold">{{ impactReport.details.removed_fields + impactReport.details.changed_fields }}</p><p class="text-sm text-[var(--ui-text-muted)]">{{ t('schema.changed_fields') }}</p></div>
          </div>
          <UAlert v-if="impactReport?.breaking" color="error" variant="soft" icon="i-lucide-triangle-alert" :title="t('schema.breaking_change')" :description="t('schema.breaking_change_hint')" />
          <div class="flex justify-end"><UButton @click="impactDialogOpen = false">{{ t('common.close') }}</UButton></div>
        </div>
      </template>
    </UModal>

    <UModal v-model:open="publishDialogOpen">
      <template #content>
        <form class="space-y-4 p-6" @submit.prevent="publishSelectedDraft">
          <div><h2 class="text-lg font-semibold">{{ t('schema.publish_confirm') }}</h2><p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ t('schema.publish_warning') }}</p></div>
          <UAlert v-if="selectedDraft?.impact?.breaking" color="error" variant="soft" icon="i-lucide-triangle-alert" :title="t('schema.breaking_change')" />
          <UFormField :label="t('schema.audit_reason')" required :hint="t('schema.reason_hint')">
            <UTextarea v-model="actionReason" class="w-full" :rows="3" />
          </UFormField>
          <UCheckbox v-model="actionConfirmed" :label="t('schema.publish_acknowledgement')" />
          <div class="flex justify-end gap-2"><UButton type="button" variant="outline" @click="publishDialogOpen = false">{{ t('common.cancel') }}</UButton><UButton type="submit" :loading="busyAction === 'publish'" :disabled="!canSubmitSensitiveAction">{{ t('schema.publish') }}</UButton></div>
        </form>
      </template>
    </UModal>

    <UModal v-model:open="retireDialogOpen">
      <template #content>
        <form class="space-y-4 p-6" @submit.prevent="retireSelectedRevision">
          <div><h2 class="text-lg font-semibold">{{ t('schema.retire_confirm') }}</h2><p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ t('schema.retire_warning') }}</p></div>
          <UFormField :label="t('schema.audit_reason')" required :hint="t('schema.reason_hint')"><UTextarea v-model="actionReason" class="w-full" :rows="3" /></UFormField>
          <UCheckbox v-model="actionConfirmed" :label="t('schema.retire_acknowledgement')" />
          <div class="flex justify-end gap-2"><UButton type="button" variant="outline" @click="retireDialogOpen = false">{{ t('common.cancel') }}</UButton><UButton type="submit" color="error" :loading="busyAction === 'retire'" :disabled="!canSubmitSensitiveAction">{{ t('schema.retire') }}</UButton></div>
        </form>
      </template>
    </UModal>

    <UModal v-model:open="deleteDialogOpen">
      <template #content>
        <div class="space-y-4 p-6">
          <div><h2 class="text-lg font-semibold">{{ t('schema.delete_confirm') }}</h2><p class="mt-1 text-sm text-[var(--ui-text-muted)]">{{ t('schema.delete_warning', { name: selectedDraft?.display_name ?? '' }) }}</p></div>
          <div class="flex justify-end gap-2"><UButton variant="outline" @click="deleteDialogOpen = false">{{ t('common.cancel') }}</UButton><UButton color="error" :loading="busyAction === 'delete'" @click="deleteSelectedDraft">{{ t('common.delete') }}</UButton></div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import type {
  SchemaChange,
  SchemaChangeKind,
  SchemaCompatibilityClass,
  SchemaImpactReport,
  SchemaRevisionRecord,
  SchemaRevisionState,
  SchemaValidationReport,
  UpdateSchemaDraftBody,
} from '@enginer/api-client';
import { capability } from '~/middleware/capability';

definePageMeta({ middleware: ['auth', capability('schema.draft')] });

const { t, locale } = useI18n();
const api = useApi();
const { catalogParams, selected: selectedCatalog } = useCatalog();
const toast = useToast();
const { hasCapability } = useCapability();

const drafts = ref<SchemaRevisionRecord[]>([]);
const revisions = ref<SchemaRevisionRecord[]>([]);
const initialLoading = ref(true);
const loadError = ref('');
const busyAction = ref('');
const activeTab = ref('drafts');
const search = ref('');
const draftStateFilter = ref('all');

const createDialogOpen = ref(false);
const editDialogOpen = ref(false);
const detailDialogOpen = ref(false);
const validationDialogOpen = ref(false);
const impactDialogOpen = ref(false);
const publishDialogOpen = ref(false);
const retireDialogOpen = ref(false);
const deleteDialogOpen = ref(false);

const selectedDraft = ref<SchemaRevisionRecord>();
const selectedRevision = ref<SchemaRevisionRecord>();
const detailRecord = ref<SchemaRevisionRecord>();
const detailLoading = ref(false);
const validationReport = ref<SchemaValidationReport>();
const impactReport = ref<SchemaImpactReport>();
const actionReason = ref('');
const actionConfirmed = ref(false);
const createError = ref('');
const editError = ref('');

const createForm = reactive({
  name: '',
  description: '',
  semantic_version: '1.1.0',
  base_revision: null as string | null,
});

const editForm = reactive({
  display_name: '',
  description: '',
  semantic_version: '',
  typesJson: '[]',
  predicatesJson: '[]',
  changes: [] as SchemaChange[],
});

const tabs = computed(() => [
  { value: 'drafts', label: `${t('schema.drafts')} (${drafts.value.length})` },
  { value: 'published', label: `${t('schema.published')} (${revisions.value.length})` },
]);
const currentRevision = computed(() => revisions.value.find(revision => revision.status === 'published'));
const latestRevision = computed(() => currentRevision.value ?? revisions.value[0]);
const readyDrafts = computed(() => drafts.value.filter(draft => draft.status === 'impact_ready').length);
const breakingDrafts = computed(() => drafts.value.filter(draft => draft.impact?.breaking).length);
const isBusy = computed(() => busyAction.value.length > 0);
const isSemanticVersion = (value: string): boolean => /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(value.trim());
const canCreateDraft = computed(() => createForm.name.trim().length > 0 && isSemanticVersion(createForm.semantic_version));
const canSaveDraft = computed(() => (
  editForm.display_name.trim().length > 0
  && isSemanticVersion(editForm.semantic_version)
  && editForm.changes.every(change => (
    change.resource_kind.trim().length > 0
    && change.resource_id.trim().length > 0
    && change.summary.trim().length > 0
  ))
));
const canSubmitSensitiveAction = computed(() => actionConfirmed.value && actionReason.value.trim().length >= 10);
const canPublishSchema = computed(() => hasCapability('schema.publish'));

const filteredDrafts = computed(() => {
  const needle = search.value.trim().toLocaleLowerCase(locale.value);
  return drafts.value.filter((draft) => {
    const stateMatches = draftStateFilter.value === 'all' || draft.status === draftStateFilter.value;
    const textMatches = !needle || [draft.display_name, draft.description, draft.semantic_version, draft.revision_id]
      .some(value => value.toLocaleLowerCase(locale.value).includes(needle));
    return stateMatches && textMatches;
  });
});

const draftStateOptions = computed(() => [
  { label: t('schema.all_states'), value: 'all' },
  { label: statusLabel('draft'), value: 'draft' },
  { label: statusLabel('validating'), value: 'validating' },
  { label: statusLabel('impact_ready'), value: 'impact_ready' },
  { label: statusLabel('failed'), value: 'failed' },
]);

const baseRevisionOptions = computed(() => [
  { label: t('schema.empty_base'), value: null },
  ...revisions.value
    .map(revision => ({
      label: `${revision.display_name} · v${revision.semantic_version} · ${statusLabel(revision.status)}`,
      value: revision.revision_id,
    })),
]);

const changeKindOptions = computed(() => (
  ['add', 'update', 'rename', 'reclassify', 'retire', 'remove'] as SchemaChangeKind[]
).map(kind => ({ label: t(`schema.change.${kind}`), value: kind })));

function statusColor(status: SchemaRevisionState): 'neutral' | 'primary' | 'success' | 'warning' | 'error' | 'info' {
  if (status === 'published') return 'success';
  if (status === 'retired') return 'neutral';
  if (status === 'failed') return 'error';
  if (status === 'impact_ready' || status === 'approved') return 'primary';
  if (status === 'validating' || status === 'publishing') return 'info';
  return 'warning';
}

function statusLabel(status: SchemaRevisionState): string {
  return t(`schema.status.${status}`);
}

function compatibilityLabel(value: SchemaCompatibilityClass): string {
  return t(`schema.compatibility.${value}`);
}

function formatDate(value?: string): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '—';
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(date);
}

function prettyJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function reloadAll(): Promise<void> {
  initialLoading.value = true;
  loadError.value = '';
  try {
    const [draftResponse, revisionResponse] = await Promise.all([
      api.schemas.listDrafts(catalogParams()),
      api.schemas.listPublished(catalogParams()),
    ]);
    drafts.value = draftResponse.data;
    revisions.value = revisionResponse.data;
  } catch (error) {
    loadError.value = errorText(error);
  } finally {
    initialLoading.value = false;
  }
}

function openCreateDialog(): void {
  createError.value = '';
  createForm.name = '';
  createForm.description = '';
  createForm.semantic_version = nextSemanticVersion(latestRevision.value?.semantic_version);
  createForm.base_revision = latestRevision.value?.revision_id ?? null;
  createDialogOpen.value = true;
}

function nextSemanticVersion(version?: string): string {
  const parts = version?.split('.').map(Number);
  if (!parts || parts.length !== 3 || parts.some(part => !Number.isSafeInteger(part) || part < 0)) return '1.0.0';
  const [major = 1, minor = 0] = parts;
  return `${major}.${minor + 1}.0`;
}

async function createDraft(): Promise<void> {
  if (!canCreateDraft.value) return;
  busyAction.value = 'create';
  createError.value = '';
  try {
    await api.schemas.createDraft({
      name: createForm.name.trim(),
      description: createForm.description.trim(),
      semantic_version: createForm.semantic_version.trim(),
      base_revision: createForm.base_revision || undefined,
    });
    createDialogOpen.value = false;
    toast.add({ title: t('schema.created_success'), color: 'success' });
    await reloadAll();
  } catch (error) {
    createError.value = errorText(error);
  } finally {
    busyAction.value = '';
  }
}

function openEditDialog(draft: SchemaRevisionRecord): void {
  selectedDraft.value = draft;
  editError.value = '';
  editForm.display_name = draft.display_name;
  editForm.description = draft.description;
  editForm.semantic_version = draft.semantic_version;
  editForm.typesJson = prettyJson(draft.types);
  editForm.predicatesJson = prettyJson(draft.predicates);
  editForm.changes = structuredClone(draft.changes);
  editDialogOpen.value = true;
}

function addChange(): void {
  editForm.changes.push({ kind: 'add', resource_kind: 'predicate', resource_id: '', summary: '', breaking: false });
}

function removeChange(index: number): void {
  editForm.changes.splice(index, 1);
}

function parseObjectArray(value: string, label: string): Record<string, unknown>[] {
  const parsed: unknown = JSON.parse(value);
  if (!Array.isArray(parsed) || parsed.some(item => item === null || typeof item !== 'object' || Array.isArray(item))) {
    throw new Error(`${label}: ${t('schema.json_array_required')}`);
  }
  return parsed as Record<string, unknown>[];
}

async function saveDraft(): Promise<void> {
  if (!selectedDraft.value) return;
  busyAction.value = 'save';
  editError.value = '';
  try {
    const body: UpdateSchemaDraftBody = {
      display_name: editForm.display_name.trim(),
      description: editForm.description.trim(),
      semantic_version: editForm.semantic_version.trim(),
      types: parseObjectArray(editForm.typesJson, t('schema.types')),
      predicates: parseObjectArray(editForm.predicatesJson, t('schema.predicates')),
      changes: editForm.changes.map(change => ({ ...change, resource_kind: change.resource_kind.trim(), resource_id: change.resource_id.trim(), summary: change.summary.trim() })),
    };
    await api.schemas.updateDraft(selectedDraft.value.revision_id, body, selectedDraft.value.etag);
    editDialogOpen.value = false;
    toast.add({ title: t('schema.saved_success'), color: 'success' });
    await reloadAll();
  } catch (error) {
    editError.value = errorText(error);
  } finally {
    busyAction.value = '';
  }
}

async function openDetail(record: SchemaRevisionRecord, source: 'draft' | 'revision'): Promise<void> {
  detailRecord.value = record;
  detailDialogOpen.value = true;
  detailLoading.value = true;
  try {
    const response = source === 'draft'
      ? await api.schemas.getDraft(record.revision_id)
      : await api.schemas.getRevision(record.revision_id);
    detailRecord.value = response.data;
  } catch (error) {
    toast.add({ title: t('common.error'), description: errorText(error), color: 'error' });
  } finally {
    detailLoading.value = false;
  }
}

async function runValidation(draft: SchemaRevisionRecord): Promise<void> {
  busyAction.value = `validate:${draft.revision_id}`;
  try {
    const response = await api.schemas.validateDraft(draft.revision_id);
    validationReport.value = response.data;
    validationDialogOpen.value = true;
    await reloadAll();
  } catch (error) {
    toast.add({ title: t('schema.validation_failed'), description: errorText(error), color: 'error' });
  } finally {
    busyAction.value = '';
  }
}

async function runImpact(draft: SchemaRevisionRecord): Promise<void> {
  busyAction.value = `impact:${draft.revision_id}`;
  try {
    const response = await api.schemas.impactDraft(draft.revision_id);
    impactReport.value = response.data;
    impactDialogOpen.value = true;
    await reloadAll();
  } catch (error) {
    toast.add({ title: t('common.error'), description: errorText(error), color: 'error' });
  } finally {
    busyAction.value = '';
  }
}

function resetSensitiveAction(): void {
  actionReason.value = '';
  actionConfirmed.value = false;
}

function openPublishDialog(draft: SchemaRevisionRecord): void {
  if (!canPublishSchema.value) return;
  selectedDraft.value = draft;
  resetSensitiveAction();
  publishDialogOpen.value = true;
}

async function publishSelectedDraft(): Promise<void> {
  if (!selectedDraft.value || !canPublishSchema.value || !canSubmitSensitiveAction.value) return;
  busyAction.value = 'publish';
  try {
    await api.schemas.publishDraft(selectedDraft.value.revision_id, selectedDraft.value.etag, actionReason.value.trim());
    publishDialogOpen.value = false;
    toast.add({ title: t('schema.published_success'), color: 'success' });
    activeTab.value = 'published';
    await reloadAll();
  } catch (error) {
    toast.add({ title: t('common.error'), description: errorText(error), color: 'error' });
  } finally {
    busyAction.value = '';
  }
}

function openRetireDialog(revision: SchemaRevisionRecord): void {
  if (!canPublishSchema.value) return;
  selectedRevision.value = revision;
  resetSensitiveAction();
  retireDialogOpen.value = true;
}

async function retireSelectedRevision(): Promise<void> {
  if (!selectedRevision.value || !canPublishSchema.value || !canSubmitSensitiveAction.value) return;
  busyAction.value = 'retire';
  try {
    await api.schemas.retireRevision(selectedRevision.value.revision_id, actionReason.value.trim(), selectedRevision.value.etag);
    retireDialogOpen.value = false;
    toast.add({ title: t('schema.retired_success'), color: 'success' });
    await reloadAll();
  } catch (error) {
    toast.add({ title: t('common.error'), description: errorText(error), color: 'error' });
  } finally {
    busyAction.value = '';
  }
}

function openDeleteDialog(draft: SchemaRevisionRecord): void {
  selectedDraft.value = draft;
  deleteDialogOpen.value = true;
}

async function deleteSelectedDraft(): Promise<void> {
  if (!selectedDraft.value) return;
  busyAction.value = 'delete';
  try {
    await api.schemas.deleteDraft(selectedDraft.value.revision_id, selectedDraft.value.etag);
    deleteDialogOpen.value = false;
    toast.add({ title: t('schema.deleted_success'), color: 'success' });
    await reloadAll();
  } catch (error) {
    toast.add({ title: t('common.error'), description: errorText(error), color: 'error' });
  } finally {
    busyAction.value = '';
  }
}

onMounted(reloadAll);
// Header catalog switch re-scopes drafts and published revisions.
watch(selectedCatalog, reloadAll);
</script>
