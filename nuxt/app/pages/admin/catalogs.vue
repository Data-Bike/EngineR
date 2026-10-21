<template>
  <div class="p-6 space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{{ t('catalog.title') }}</h1>
        <p class="text-sm text-[var(--ui-text-muted)]">{{ t('catalog.desc') }}</p>
      </div>
      <UButton icon="i-lucide-plus" @click="openCreateDialog">
        {{ t('catalog.create') }}
      </UButton>
    </div>

    <div v-if="loading" class="text-[var(--ui-text-muted)]">{{ t('common.loading') }}</div>
    <div v-else-if="catalogs.length === 0" class="text-[var(--ui-text-muted)]">
      {{ t('catalog.empty') }}
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <UCard v-for="cat in catalogs" :key="cat.public_id">
        <div class="space-y-2">
          <div class="flex items-start justify-between gap-2">
            <div>
              <div class="flex items-center gap-2">
                <p class="font-medium">{{ cat.name }}</p>
                <UBadge :color="cat.status === 'active' ? 'success' : 'neutral'" variant="subtle">
                  {{ t(`catalog.${cat.status}`) }}
                </UBadge>
              </div>
              <p class="text-xs font-mono text-[var(--ui-text-muted)]">{{ cat.alias }}</p>
            </div>
            <div class="flex gap-1">
              <UButton size="sm" variant="outline" @click="editCatalog(cat)">
                {{ t('catalog.edit') }}
              </UButton>
              <UButton
                v-if="cat.status === 'active'"
                size="sm"
                variant="ghost"
                color="error"
                icon="i-lucide-archive"
                :aria-label="t('catalog.retire')"
                :title="t('catalog.retire')"
                @click="askRetire(cat)"
              />
            </div>
          </div>
          <p v-if="cat.description" class="text-sm text-[var(--ui-text-muted)]">
            {{ cat.description }}
          </p>
          <div class="flex gap-4 text-xs text-[var(--ui-text-muted)]">
            <span>{{ t('catalog.entities') }}: {{ cat.entity_count }}</span>
            <span>{{ t('catalog.types') }}: {{ cat.type_count }}</span>
          </div>
        </div>
      </UCard>
    </div>

    <UModal v-model:open="showDialog">
      <template #header>{{ editing ? t('catalog.edit') : t('catalog.create') }}</template>
      <template #body>
        <div class="space-y-4">
          <UFormField :label="t('catalog.alias')" :hint="t('catalog.alias_hint')">
            <UInput v-model="formData.alias" :disabled="editing" class="w-full" placeholder="hr" />
          </UFormField>
          <UFormField :label="t('catalog.name')">
            <UInput v-model="formData.name" class="w-full" placeholder="HR" />
          </UFormField>
          <UFormField :label="t('catalog.description')">
            <UTextarea v-model="formData.description" class="w-full" />
          </UFormField>
        </div>
      </template>
      <template #footer>
        <UButton variant="ghost" @click="showDialog = false">{{ t('catalog.cancel') }}</UButton>
        <UButton :loading="saving" @click="saveCatalog">{{ t('catalog.save') }}</UButton>
      </template>
    </UModal>

    <UModal v-model:open="showRetire">
      <template #header>{{ t('catalog.retire') }}</template>
      <template #body>
        <p>{{ t('catalog.retire_confirm') }}</p>
        <p v-if="retireTarget" class="mt-2 font-mono text-sm">{{ retireTarget.alias }}</p>
      </template>
      <template #footer>
        <UButton variant="ghost" @click="showRetire = false">{{ t('catalog.cancel') }}</UButton>
        <UButton color="error" :loading="retiring" @click="retireCatalog">
          {{ t('catalog.retire') }}
        </UButton>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import type { CatalogRecord } from '@enginer/api-client';
import { capability } from '~/middleware/capability';

definePageMeta({
  middleware: ['auth', capability('admin.pages.access')],
});

const { t, te } = useI18n();
const api = useApi();
const toast = useToast();
const authStore = useAuthStore();

const loading = ref(false);
const saving = ref(false);
const showDialog = ref(false);
const editing = ref(false);
const catalogs = ref<CatalogRecord[]>([]);
const formData = ref({ alias: '', name: '', description: '' });
const showRetire = ref(false);
const retiring = ref(false);
const retireTarget = ref<CatalogRecord | null>(null);

async function fetchCatalogs() {
  loading.value = true;
  try {
    const res = await api.catalogs.list();
    catalogs.value = res.data ?? [];
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    loading.value = false;
  }
}

function openCreateDialog() {
  editing.value = false;
  formData.value = { alias: '', name: '', description: '' };
  showDialog.value = true;
}

function editCatalog(cat: CatalogRecord) {
  editing.value = true;
  formData.value = { alias: cat.alias, name: cat.name, description: cat.description };
  showDialog.value = true;
}

async function saveCatalog() {
  saving.value = true;
  try {
    if (editing.value) {
      await api.catalogs.update(formData.value.alias, {
        name: formData.value.name,
        description: formData.value.description,
      });
      toast.add({ title: t('catalog.update_success'), color: 'success' });
    } else {
      await api.catalogs.create({
        alias: formData.value.alias,
        name: formData.value.name,
        description: formData.value.description || undefined,
      });
      toast.add({ title: t('catalog.create_success'), color: 'success' });
    }
    showDialog.value = false;
    await fetchCatalogs();
    // /me/context.catalogs feeds the header selector — refresh it so a
    // newly created (or renamed) catalog shows up without a reload.
    await authStore.fetchContext();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    saving.value = false;
  }
}

function askRetire(cat: CatalogRecord) {
  retireTarget.value = cat;
  showRetire.value = true;
}

async function retireCatalog() {
  if (!retireTarget.value) return;
  retiring.value = true;
  try {
    await api.catalogs.retire(retireTarget.value.alias);
    toast.add({ title: t('catalog.retire_success'), color: 'success' });
    showRetire.value = false;
    retireTarget.value = null;
    await fetchCatalogs();
    await authStore.fetchContext();
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: displayError(e, t, te), color: 'error' });
  } finally {
    retiring.value = false;
  }
}

onMounted(fetchCatalogs);
</script>
