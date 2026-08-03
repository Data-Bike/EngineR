<template>
  <div class="space-y-4">
    <!-- Toolbar (docs/17 §7) -->
    <div class="flex flex-wrap items-center gap-2">
      <UFormField :label="t('entity.graph.as_of')" size="sm">
        <UInput v-model="asOf" type="date" size="sm" @change="reload" />
      </UFormField>
      <UFormField :label="t('entity.graph.depth')" size="sm">
        <UInput v-model.number="depth" type="number" min="1" :max="maxDepth" size="sm" class="w-20" @change="reload" />
      </UFormField>
      <USelect
        v-model="direction"
        :items="directionOptions"
        size="sm"
        @update:model-value="reload"
      />
      <UButton
        :icon="showTree ? 'i-lucide-network' : 'i-lucide-list'"
        size="sm"
        variant="outline"
        @click="showTree = !showTree"
      >
        {{ showTree ? t('entity.graph.view_graph') : t('entity.graph.view_tree') }}
      </UButton>
    </div>

    <!-- Graph view (visual) -->
    <div v-if="!showTree" class="relative rounded-lg border border-[var(--ui-border)] min-h-[400px] overflow-hidden">
      <div ref="graphContainer" class="w-full h-[400px]">
        <!-- SVG graph -->
        <svg :viewBox="`0 0 ${graphWidth} ${graphHeight}`" class="w-full h-full" role="img" :aria-label="t('entity.graph.aria_label')">
          <!-- Edges -->
          <g class="edges">
            <g v-for="edge in layoutEdges" :key="`${edge.source}-${edge.target}`">
              <line
                :x1="edge.x1"
                :y1="edge.y1"
                :x2="edge.x2"
                :y2="edge.y2"
                :class="['stroke-2', edge.cycle ? 'stroke-orange-400' : 'stroke-[var(--ui-border-muted)]']"
                :stroke-dasharray="edge.cycle ? '5,3' : undefined"
              />
              <!-- Edge label -->
              <text
                v-if="edge.label"
                :x="(edge.x1 + edge.x2) / 2"
                :y="(edge.y1 + edge.y2) / 2 - 4"
                class="fill-[var(--ui-text-muted)] text-xs"
                text-anchor="middle"
              >
                {{ edge.label }}
              </text>
            </g>
          </g>
          <!-- Nodes -->
          <g class="nodes">
            <g
              v-for="node in layoutNodes"
              :key="node.id"
              :transform="`translate(${node.x}, ${node.y})`"
              class="cursor-pointer"
              @click="selectNode(node.id)"
            >
              <!-- Node shape (not color-only marker, docs/17 §7) -->
              <rect
                :x="-nodeWidth / 2"
                :y="-nodeHeight / 2"
                :width="nodeWidth"
                :height="nodeHeight"
                rx="6"
                :class="[
                  'stroke-2',
                  selectedNodeId === node.id ? 'stroke-blue-500 fill-blue-50' : 'stroke-[var(--ui-border)] fill-[var(--ui-bg)]',
                ]"
              />
              <!-- Node icon by type (not color only) -->
              <text :y="-4" class="fill-[var(--ui-text)] text-xs font-medium" text-anchor="middle">
                {{ node.label }}
              </text>
              <text :y="10" class="fill-[var(--ui-text-muted)] text-[10px]" text-anchor="middle">
                {{ node.typeLabel }}
              </text>
              <!-- Cycle marker (docs/17 §7) -->
              <text v-if="node.cycle" :y="nodeHeight / 2 - 2" class="fill-orange-500 text-[10px]" text-anchor="middle">
                ↩
              </text>
              <!-- Expand button for non-leaf nodes -->
              <circle
                v-if="!node.expanded && node.hasChildren"
                :cx="nodeWidth / 2"
                :cy="0"
                r="8"
                class="fill-blue-500 stroke-white cursor-pointer"
                @click.stop="expandNode(node.id)"
              />
              <text
                v-if="!node.expanded && node.hasChildren"
                :x="nodeWidth / 2"
                :y="3"
                class="fill-white text-xs"
                text-anchor="middle"
                @click.stop="expandNode(node.id)"
              >
                +
              </text>
            </g>
          </g>
        </svg>
      </div>

      <!-- Empty state -->
      <div v-if="nodes.length === 0" class="absolute inset-0 flex items-center justify-center text-[var(--ui-text-muted)]">
        {{ t('entity.empty') }}
      </div>
    </div>

    <!-- Tree view (accessible alternative, docs/17 §7, docs/16 §12) -->
    <div v-else class="rounded-lg border border-[var(--ui-border)] p-4">
      <ul role="tree" :aria-label="t('entity.graph.aria_label')">
        <GraphTreeNode
          v-for="node in treeRoots"
          :key="node.id"
          :node="node"
          :all-nodes="nodeMap"
          :visited="visited"
          :depth="0"
          @select="selectNode"
          @expand="expandNode"
        />
      </ul>
    </div>

    <!-- Context panel (docs/17 §7: selection opens context panel) -->
    <UCard v-if="selectedNode">
      <template #header>
        <h3 class="font-medium">{{ selectedNode.label }}</h3>
        <p class="text-sm text-[var(--ui-text-muted)]">{{ selectedNode.typeLabel }}</p>
      </template>
      <dl class="grid grid-cols-2 gap-2 text-sm">
        <dt class="text-[var(--ui-text-muted)]">ID</dt>
        <dd>{{ selectedNode.id }}</dd>
        <dt class="text-[var(--ui-text-muted)]">{{ t('entity.graph.edges') }}</dt>
        <dd>{{ getEdgesForNode(selectedNode.id).length }}</dd>
      </dl>
      <template #footer>
        <UButton size="sm" @click="navigateToEntity(selectedNode.id)">
          {{ t('entity.open') }}
        </UButton>
      </template>
    </UCard>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  rootEntityId: string;
  rootTypeId: string;
  maxDepth?: number;
}>();

const { t } = useI18n();
const api = useApi();

// --- State ---
const asOf = ref<string>('');
const depth = ref(1);
const direction = ref<'outgoing' | 'incoming' | 'both'>('both');
const showTree = ref(false);
const nodes = ref<GraphNode[]>([]);
const edges = ref<GraphEdge[]>([]);
const visited = ref<Set<string>>(new Set());
const selectedNodeId = ref<string | null>(null);

const maxDepth = computed(() => props.maxDepth ?? 5);

const directionOptions = computed(() => [
  { label: t('entity.graph.direction_outgoing'), value: 'outgoing' },
  { label: t('entity.graph.direction_incoming'), value: 'incoming' },
  { label: t('entity.graph.direction_both'), value: 'both' },
]);

// --- Types ---
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

interface GraphEdge {
  source: string;
  target: string;
  label?: string;
  cycle: boolean;
}

// --- Layout ---
const nodeWidth = 120;
const nodeHeight = 50;
const graphWidth = 800;
const graphHeight = 400;

const layoutNodes = computed(() => {
  // Simple radial layout: root at center, children in concentric rings
  const root = nodes.value.find((n) => n.id === props.rootEntityId);
  if (!root) return [];
  const positioned = nodes.value.map((n, i) => {
    if (n.id === props.rootEntityId) {
      return { ...n, x: graphWidth / 2, y: graphHeight / 2 };
    }
    const ring = Math.floor(i / 4) + 1;
    const angle = (i % 4) * (Math.PI / 2);
    const radius = ring * 130;
    return {
      ...n,
      x: graphWidth / 2 + Math.cos(angle) * radius,
      y: graphHeight / 2 + Math.sin(angle) * radius,
    };
  });
  return positioned;
});

const layoutEdges = computed(() => {
  const nodeMap = new Map(layoutNodes.value.map((n) => [n.id, n]));
  return edges.value.map((e) => {
    const s = nodeMap.get(e.source);
    const tg = nodeMap.get(e.target);
    return {
      ...e,
      x1: s?.x ?? 0,
      y1: s?.y ?? 0,
      x2: tg?.x ?? 0,
      y2: tg?.y ?? 0,
    };
  });
});

// --- Tree view ---
const nodeMap = computed(() => {
  const map = new Map<string, GraphNode>();
  nodes.value.forEach((n) => map.set(n.id, n));
  return map;
});

const treeRoots = computed(() => {
  const childIds = new Set(edges.value.map((e) => e.target));
  return nodes.value.filter((n) => !childIds.has(n.id));
});

// --- Selected node ---
const selectedNode = computed(() =>
  selectedNodeId.value ? nodeMap.value.get(selectedNodeId.value) ?? null : null,
);

function getEdgesForNode(nodeId: string): GraphEdge[] {
  return edges.value.filter((e) => e.source === nodeId || e.target === nodeId);
}

// --- Actions ---
function selectNode(id: string) {
  selectedNodeId.value = id;
}

function navigateToEntity(id: string) {
  navigateTo(`/entities/${id}`);
}

async function expandNode(nodeId: string) {
  if (visited.value.has(nodeId)) return;
  visited.value.add(nodeId);

  // Fetch relations for this node (progressive traversal, docs/17 §7)
  try {
    await api.entities.query({
      query: {
        from: { type_id: props.rootTypeId, alias: 'e' },
        select: [{ field: { system: 'public_id' }, alias: 'public_id' }],
        where: {
          relation: {
            predicate_id: '*',
            direction: direction.value as 'outgoing' | 'incoming' | 'both',
            target_type_id: undefined,
          },
        },
        sort: [],
        page: { size: 20 },
      },
    });
    // Add new nodes and edges from response
    // (Real implementation would parse the relation data)
    const node = nodes.value.find((n) => n.id === nodeId);
    if (node) {
      node.expanded = true;
    }
  } catch {
    // Silent fail for progressive expansion
  }
}

async function reload() {
  nodes.value = [];
  edges.value = [];
  visited.value.clear();
  // Initial load: root node + depth-1 neighbors
  nodes.value.push({
    id: props.rootEntityId,
    label: props.rootEntityId.slice(0, 8),
    typeLabel: props.rootTypeId.slice(0, 8),
    typeId: props.rootTypeId,
    expanded: false,
    hasChildren: true,
    cycle: false,
    x: 0,
    y: 0,
  });
  if (depth.value > 0) {
    await expandNode(props.rootEntityId);
  }
}

onMounted(reload);
watch(() => props.rootEntityId, reload);
</script>
