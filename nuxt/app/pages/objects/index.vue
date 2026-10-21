<template>
  <div class="p-4 sm:p-6 space-y-6 max-w-full overflow-x-hidden">
    <div>
      <h1 class="text-2xl font-bold">{{ t('nav.objects') }}</h1>
      <p class="text-[var(--ui-text-muted)] mt-1">{{ t('objects.subtitle') }}</p>
    </div>

    <div v-if="loading" class="py-8 text-center text-[var(--ui-text-muted)]">{{ t('common.loading') }}</div>
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <NuxtLink
        v-for="typ in types"
        :key="`${typ.catalog ?? ''}:${typ.id}`"
        :to="typeLink(typ)"
        class="block rounded-lg focus-visible:outline-2 focus-visible:outline-[var(--ui-primary)]"
      >
        <UCard class="h-full hover:border-[var(--ui-primary)] transition-colors">
          <template #header>
            <div class="flex items-center gap-3">
              <UIcon :name="typeIcon(typ.id)" class="text-2xl text-[var(--ui-primary)]" />
              <div>
                <h3 class="font-semibold">{{ typ.display_name }}</h3>
                <p class="text-xs text-[var(--ui-text-muted)] font-mono">
                  {{ typ.id }}<template v-if="typ.catalog"> · {{ typ.catalog }}</template>
                </p>
              </div>
            </div>
          </template>
          <div class="flex items-center justify-between text-sm">
            <span class="text-[var(--ui-text-muted)]">{{ t('objects.count') }}: {{ typ.count ?? '—' }}</span>
            <UBadge variant="subtle" size="sm">{{ t(`objects.status.${typ.status}`) }}</UBadge>
          </div>
        </UCard>
      </NuxtLink>
    </div>
  </div>
</template>

<script setup lang="ts">
import { capability } from '~/middleware/capability';
import { ref, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';

definePageMeta({ middleware: ['auth', capability('entity.read')] });

const { t, te } = useI18n();
const toast = useToast();
const api = useApi();
const { catalogParams, selected } = useCatalog();

interface TypeRow {
  id: string;
  name: string;
  display_name: string;
  status: string;
  version: number;
  catalog?: string;
  entity_count?: number;
  count?: number;
}

const types = ref<TypeRow[]>([]);
const loading = ref(false);

function typeIcon(typeId: string): string {
  switch (typeId) {
    case 'person': return 'i-lucide-user';
    case 'legal_entity': return 'i-lucide-building-2';
    case 'request': return 'i-lucide-file-text';
    case 'address': return 'i-lucide-map-pin';
    case 'contact_point': return 'i-lucide-phone';
    case 'document': return 'i-lucide-file';
    case 'license': return 'i-lucide-badge-check';
    case 'contract': return 'i-lucide-file-signature';
    case 'collateral': return 'i-lucide-shield';
    default: return 'i-lucide-box';
  }
}

function typeLink(typ: TypeRow) {
  return {
    path: `/objects/${typ.id}`,
    query: typ.catalog ? { catalog: typ.catalog } : {},
  };
}

async function fetchTypes() {
  loading.value = true;
  try {
    const res = await api.schemas.listTypes(catalogParams());
    const schemaTypes = (res.data || []) as TypeRow[];
    types.value = schemaTypes.map((schemaType) => ({
      ...schemaType,
      count: schemaType.entity_count ?? schemaType.count ?? 0,
    }));
  } catch (err) {
    toast.add({ title: t('common.error'), description: displayError(err, t, te), color: 'error' });
  } finally {
    loading.value = false;
  }
}

onMounted(fetchTypes);
// The header catalog selector scopes this listing — refetch on switch.
watch(selected, fetchTypes);
</script>
