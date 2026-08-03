<template>
  <li role="treeitem" :aria-expanded="hasChildren ? node.expanded : undefined" :aria-selected="selected">
    <div class="flex items-center gap-1 py-1" :style="{ paddingLeft: `${depth * 1.5}rem` }">
      <!-- Expand/collapse button -->
      <button
        v-if="hasChildren"
        @click="expand"
        class="shrink-0"
        :aria-label="node.expanded ? t('entity.graph.collapse') : t('entity.graph.expand')"
      >
        <UIcon :name="node.expanded ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'" class="text-sm" />
      </button>
      <span v-else class="w-4 shrink-0" />

      <!-- Node label -->
      <button
        @click="select"
        class="text-sm hover:underline text-left"
        :class="{ 'font-bold': selected }"
      >
        {{ node.label }}
        <span class="text-[var(--ui-text-muted)] text-xs">({{ node.typeLabel }})</span>
        <span v-if="isCycle" class="text-orange-500 text-xs ml-1">↩</span>
      </button>
    </div>

    <!-- Children -->
    <ul v-if="node.expanded && hasChildren" role="group">
      <GraphTreeNode
        v-for="childId in childIds"
        :key="childId"
        :node="allNodes.get(childId)!"
        :all-nodes="allNodes"
        :visited="visited"
        :depth="depth + 1"
        @select="$emit('select', $event)"
        @expand="$emit('expand', $event)"
      />
    </ul>
  </li>
</template>

<script setup lang="ts">
interface GraphNode {
  id: string;
  label: string;
  typeLabel: string;
  typeId: string;
  expanded: boolean;
  hasChildren: boolean;
  cycle: boolean;
  x: number;
  y: number;
}

const props = defineProps<{
  node: GraphNode;
  allNodes: Map<string, GraphNode>;
  visited: Set<string>;
  depth: number;
}>();

const emit = defineEmits<{
  select: [id: string];
  expand: [id: string];
}>();

const { t } = useI18n();

// Child IDs would come from edges — for now, derive from visited set
const childIds = computed<string[]>(() => {
  // In real implementation, this would look up edges where source = node.id
  return [];
});

const hasChildren = computed(() => props.node.hasChildren);
const selected = computed(() => false); // Would be tracked by parent
const isCycle = computed(() => props.visited.has(props.node.id));

function select() {
  emit('select', props.node.id);
}

function expand() {
  emit('expand', props.node.id);
}
</script>
