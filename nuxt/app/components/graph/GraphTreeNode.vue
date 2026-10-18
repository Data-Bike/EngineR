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
        :edges="edges"
        :visited="visited"
        :selected-node-id="selectedNodeId"
        :ancestors="[...ancestors, node.id]"
        :depth="depth + 1"
        @select="$emit('select', $event)"
        @expand="$emit('expand', $event)"
      />
    </ul>
  </li>
</template>

<script setup lang="ts">
import { computed } from 'vue';

interface GraphNode {
  id: string;
  label: string;
  typeLabel: string;
  typeId: string;
  expanded: boolean;
  hasChildren: boolean;
  cycle: boolean;
  x?: number;
  y?: number;
  r?: number;
  status?: string;
  virtual?: boolean;
}

interface GraphEdge {
  source: string;
  target: string;
}

const props = defineProps<{
  node: GraphNode;
  allNodes: Map<string, GraphNode>;
  edges: GraphEdge[];
  visited: Set<string>;
  selectedNodeId: string | null;
  ancestors: string[];
  depth: number;
}>();

const emit = defineEmits<{
  select: [id: string];
  expand: [id: string];
}>();

const { t } = useI18n();

const childIds = computed<string[]>(() => {
  const result = new Set<string>();
  for (const edge of props.edges) {
    const candidate = edge.source === props.node.id
      ? edge.target
      : edge.target === props.node.id
        ? edge.source
        : undefined;
    if (candidate && !props.ancestors.includes(candidate) && props.allNodes.has(candidate)) {
      result.add(candidate);
    }
  }
  return [...result];
});

const hasChildren = computed(() => childIds.value.length > 0 || props.node.hasChildren);
const selected = computed(() => props.selectedNodeId === props.node.id);
const isCycle = computed(() => props.node.cycle);

function select() {
  emit('select', props.node.id);
}

function expand() {
  emit('expand', props.node.id);
}
</script>
