<template>
  <UModal :open="open" @update:open="$emit('update:open', $event)">
    <template #content>
      <div class="p-6 space-y-4">
        <h2 class="text-lg font-bold">{{ t('entity.duplicate.title') }}</h2>

        <!-- Exact matches (docs/17 §5) -->
        <div v-if="exactMatches.length > 0" class="space-y-2">
          <p class="text-sm font-medium">{{ t('entity.duplicate.exact_match') }}</p>
          <div
            v-for="match in exactMatches"
            :key="match.public_id"
            class="rounded-md border border-orange-300 bg-orange-50 p-3"
          >
            <div class="flex items-center justify-between">
              <div>
                <p class="font-medium">{{ match.display_name ?? match.public_id }}</p>
                <p class="text-sm text-[var(--ui-text-muted)]">{{ match.type_id }}</p>
              </div>
              <div class="flex gap-2">
                <UButton size="sm" @click="$emit('open-existing', match.public_id)">
                  {{ t('entity.duplicate.open_existing') }}
                </UButton>
                <UButton
                  v-if="identityMode !== 'reject'"
                  size="sm"
                  variant="outline"
                  @click="$emit('create-distinct')"
                >
                  {{ t('entity.duplicate.create_distinct') }}
                </UButton>
              </div>
            </div>
          </div>
        </div>

        <!-- Fuzzy matches (docs/17 §5) -->
        <div v-if="fuzzyMatches.length > 0" class="space-y-2">
          <p class="text-sm font-medium">{{ t('entity.duplicate.fuzzy_match') }}</p>
          <div
            v-for="match in fuzzyMatches"
            :key="match.public_id"
            class="rounded-md border border-[var(--ui-border)] p-3"
          >
            <div class="flex items-center justify-between">
              <div>
                <p class="font-medium">{{ match.display_name ?? match.public_id }}</p>
                <p class="text-sm text-[var(--ui-text-muted)]">
                  {{ match.type_id }} · {{ t('entity.duplicate.score') }}: {{ match.score }}
                </p>
                <p v-if="match.explanation" class="text-sm text-[var(--ui-text-muted)] mt-1">
                  {{ match.explanation }}
                </p>
              </div>
              <UButton size="sm" variant="outline" @click="$emit('open-existing', match.public_id)">
                {{ t('entity.duplicate.open_existing') }}
              </UButton>
            </div>
          </div>
          <p class="text-xs text-[var(--ui-text-muted)]">
            {{ t('entity.duplicate.no_auto_merge') }}
          </p>
        </div>

        <!-- No matches -->
        <div v-if="matches.length === 0" class="text-center py-4 text-[var(--ui-text-muted)]">
          {{ t('entity.duplicate.no_matches') }}
        </div>

        <!-- Actions -->
        <div class="flex justify-end gap-2 pt-2 border-t border-[var(--ui-border)]">
          <UButton variant="outline" @click="$emit('cancel')">
            {{ t('common.cancel') }}
          </UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

interface DuplicateMatch {
  public_id: string;
  display_name?: string;
  type_id: string;
  match_type: 'exact' | 'fuzzy';
  score?: number;
  explanation?: string;
}

const props = defineProps<{
  open: boolean;
  matches: DuplicateMatch[];
  identityMode: 'return_existing' | 'reject' | 'create_candidate';
}>();

defineEmits<{
  'update:open': [value: boolean];
  'open-existing': [publicId: string];
  'create-distinct': [];
  'cancel': [];
}>();

const { t } = useI18n();

const exactMatches = computed(() =>
  props.matches.filter((m) => m.match_type === 'exact'),
);

const fuzzyMatches = computed(() =>
  props.matches.filter((m) => m.match_type === 'fuzzy'),
);
</script>
