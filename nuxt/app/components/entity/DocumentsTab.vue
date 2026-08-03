<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h3 class="font-medium">{{ t('entity.tabs.documents') }}</h3>
      <UButton size="sm" icon="i-lucide-upload" @click="showUpload = true">
        {{ t('entity.documents.upload') }}
      </UButton>
    </div>

    <!-- Document items (docs/17 §6: items with title, qualifiers, validity, source) -->
    <div v-if="documents.length === 0" class="text-center py-8 text-[var(--ui-text-muted)]">
      {{ t('entity.empty') }}
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="doc in documents"
        :key="doc.id"
        class="rounded-md border border-[var(--ui-border)] p-3 hover:border-[var(--ui-border-muted)]"
      >
        <div class="flex items-start justify-between">
          <div class="flex items-start gap-3 flex-1">
            <!-- Document type icon (not color-only, docs/17 §7) -->
            <UIcon :name="docIcon(doc.kind)" class="text-2xl text-[var(--ui-text-muted)] shrink-0" />
            <div class="flex-1">
              <!-- Title -->
              <p class="font-medium">{{ doc.title }}</p>
              <!-- Qualifiers (docs/17 §6) -->
              <div v-if="doc.qualifiers && doc.qualifiers.length > 0" class="flex flex-wrap gap-1 mt-1">
                <UBadge
                  v-for="qual in doc.qualifiers"
                  :key="qual.label"
                  variant="subtle"
                  size="sm"
                >
                  {{ qual.label }}: {{ qual.value }}
                </UBadge>
              </div>
              <!-- Validity (docs/17 §6) -->
              <p v-if="doc.validFrom || doc.validTo" class="text-xs text-[var(--ui-text-muted)] mt-1">
                <span v-if="doc.validFrom">{{ t('entity.relations.valid_from') }}: {{ doc.validFrom }}</span>
                <span v-if="doc.validFrom && doc.validTo"> · </span>
                <span v-if="doc.validTo">{{ t('entity.relations.valid_to') }}: {{ doc.validTo }}</span>
              </p>
              <!-- Source (docs/17 §6) -->
              <p v-if="doc.source" class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ t('entity.relations.source') }}: {{ doc.source }}
              </p>
              <!-- File metadata -->
              <p class="text-xs text-[var(--ui-text-muted)] mt-1">
                {{ doc.fileName }} · {{ formatFileSize(doc.sizeBytes) }}
              </p>
            </div>
          </div>
          <!-- Actions -->
          <div class="flex gap-1">
            <UButton size="xs" variant="ghost" icon="i-lucide-download" @click="downloadDoc(doc)" />
            <UButton size="xs" variant="ghost" icon="i-lucide-eye" @click="previewDoc(doc)" />
            <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash" @click="removeDoc(doc)" />
          </div>
        </div>
      </div>
    </div>

    <!-- Upload dialog -->
    <UModal v-model:open="showUpload">
      <template #content>
        <div class="p-6 space-y-4">
          <h2 class="text-lg font-bold">{{ t('entity.documents.upload') }}</h2>
          <UFormField :label="t('entity.documents.title')">
            <UInput v-model="newDocTitle" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.documents.kind')">
            <USelect v-model="newDocKind" :items="docKindOptions" class="w-full" />
          </UFormField>
          <UFormField :label="t('entity.documents.file')">
            <UInput type="file" class="w-full" @change="onFileSelect" />
          </UFormField>
          <div class="flex justify-end gap-2">
            <UButton variant="outline" @click="showUpload = false">{{ t('common.cancel') }}</UButton>
            <UButton @click="uploadDoc" :loading="uploading">{{ t('common.save') }}</UButton>
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';

const props = defineProps<{
  entityId: string;
}>();

const { t } = useI18n();
const toast = useToast();

interface DocQualifier {
  label: string;
  value: string;
}

interface DocumentItem {
  id: string;
  title: string;
  kind: string;
  fileName: string;
  sizeBytes: number;
  qualifiers: DocQualifier[];
  validFrom?: string;
  validTo?: string;
  source?: string;
}

const documents = ref<DocumentItem[]>([]);
const showUpload = ref(false);
const newDocTitle = ref('');
const newDocKind = ref('passport');
const newFile = ref<File | null>(null);
const uploading = ref(false);

const docKindOptions = computed(() => [
  { label: t('entity.documents.kind_passport'), value: 'passport' },
  { label: t('entity.documents.kind_registration'), value: 'registration' },
  { label: t('entity.documents.kind_tax'), value: 'tax' },
  { label: t('entity.documents.kind_contract'), value: 'contract' },
  { label: t('entity.documents.kind_other'), value: 'other' },
]);

function docIcon(kind: string): string {
  const icons: Record<string, string> = {
    passport: 'i-lucide-id-card',
    registration: 'i-lucide-file-text',
    tax: 'i-lucide-receipt',
    contract: 'i-lucide-file-signature',
    other: 'i-lucide-file',
  };
  return icons[kind] ?? 'i-lucide-file';
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function onFileSelect(event: Event) {
  const target = event.target as HTMLInputElement;
  newFile.value = target.files?.[0] ?? null;
}

interface ApiDocument {
  id: string;
  title: string;
  kind: string;
  file_name: string;
  size_bytes: number;
  qualifiers?: Array<{ label: string; value: string }>;
  valid_from?: string;
  valid_to?: string;
  source?: string;
}

async function fetchDocuments() {
  const config = useRuntimeConfig();
  const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
  try {
    const res = await $fetch<{ data: ApiDocument[] }>(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/documents`, {
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    documents.value = (res.data ?? []).map((doc) => ({
      id: doc.id,
      title: doc.title,
      kind: doc.kind,
      fileName: doc.file_name,
      sizeBytes: doc.size_bytes,
      qualifiers: doc.qualifiers ?? [],
      validFrom: doc.valid_from,
      validTo: doc.valid_to,
      source: doc.source,
    }));
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
    documents.value = [];
  }
}

async function uploadDoc() {
  if (!newFile.value) return;
  uploading.value = true;
  try {
    const config = useRuntimeConfig();
    const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
    const formData = new FormData();
    formData.append('file', newFile.value);
    formData.append('title', newDocTitle.value);
    formData.append('kind', newDocKind.value);
    await $fetch(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/documents`, {
      method: 'POST',
      credentials: 'include',
      headers: { Accept: 'application/json' },
      body: formData,
    });
    toast.add({ title: t('entity.documents.upload'), color: 'success' });
    showUpload.value = false;
    newDocTitle.value = '';
    newFile.value = null;
    await fetchDocuments();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  } finally {
    uploading.value = false;
  }
}

async function downloadDoc(doc: DocumentItem) {
  const config = useRuntimeConfig();
  const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
  try {
    await $fetch(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/documents/${doc.id}/download`, {
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('entity.documents.download', 'Download'), color: 'success' });
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

function previewDoc(_doc: DocumentItem) {
  // Would open preview modal
}

async function removeDoc(doc: DocumentItem) {
  const config = useRuntimeConfig();
  const baseUrl = (config.public.apiBaseUrl as string) || '/api/v1';
  try {
    await $fetch(`${baseUrl.replace(/\/$/, '')}/entities/${props.entityId}/documents/${doc.id}`, {
      method: 'DELETE',
      credentials: 'include',
      headers: { Accept: 'application/json' },
    });
    toast.add({ title: t('entity.documents.removed', 'Removed'), color: 'success' });
    await fetchDocuments();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

onMounted(fetchDocuments);
</script>
