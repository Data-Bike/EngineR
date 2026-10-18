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
      <!-- Zoom controls -->
      <div class="flex items-center gap-1 ml-auto">
        <UButton icon="i-lucide-zoom-out" size="xs" variant="outline" :aria-label="'-'" @click="zoomOut" />
        <span class="text-xs text-[var(--ui-text-muted)] w-10 text-center">{{ Math.round(zoom * 100) }}%</span>
        <UButton icon="i-lucide-zoom-in" size="xs" variant="outline" :aria-label="'+'" @click="zoomIn" />
        <UButton icon="i-lucide-maximize" size="xs" variant="ghost" :aria-label="'fit'" @click="zoomFit" />
      </div>
    </div>

    <!-- Graph view (visual) -->
    <div class="relative rounded-lg border border-[var(--ui-border)] min-h-[560px] overflow-hidden bg-[var(--ui-bg-muted)]/20" :class="{ hidden: showTree }">
      <div ref="graphContainer" class="min-w-full min-h-[560px]">
        <!-- SVG graph: force-directed bubbles -->
        <svg
          ref="svgEl"
          :width="graphWidth * zoom"
          :height="graphHeight * zoom"
          :viewBox="`${viewMinX} ${viewMinY} ${graphWidth} ${graphHeight}`"
          role="img"
          :aria-label="t('entity.graph.aria_label')"
          class="select-none touch-none"
          @pointermove="onDragMove"
          @pointerup="endDrag"
          @pointerleave="endDrag"
        >
          <defs>
            <marker id="graph-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M 0 0 L 10 5 L 0 10 z" class="fill-[var(--ui-border-muted)]" />
            </marker>
          </defs>
          <!-- Edges -->
          <g class="edges">
            <g v-for="edge in layoutEdges" :key="edge.id">
              <path
                :d="edge.d"
                fill="none"
                :class="['stroke-2', edge.cycle || edge.virtual ? 'stroke-orange-400' : 'stroke-[var(--ui-border-muted)]']"
                :stroke-dasharray="edge.cycle || edge.virtual ? '5,3' : undefined"
                marker-end="url(#graph-arrow)"
              />
              <!-- Edge label (offset perpendicular so parallel edges stay readable) -->
              <text
                v-if="edge.label && (!denseGraph || edge.virtual)"
                :x="edge.labelX"
                :y="edge.labelY"
                class="fill-[var(--ui-text-muted)] text-xs"
                text-anchor="middle"
                @dblclick.stop="edge.virtual ? handleVirtualEdgeClick(edge) : undefined"
              >
                {{ edge.label }}
                <title v-if="edge.virtual">{{ t('entity.graph.virtual_hint') }}</title>
              </text>
              <!-- Virtual edges are clickable targets themselves -->
              <path
                v-if="edge.virtual"
                :d="edge.d"
                fill="none"
                class="stroke-transparent cursor-pointer"
                :stroke-width="14"
                @dblclick.stop="handleVirtualEdgeClick(edge)"
                @click.stop="selectVirtualEdge(edge)"
              />
            </g>
          </g>
          <!-- Nodes -->
          <g class="nodes">
            <g
              v-for="node in layoutNodes"
              :key="node.id"
              :transform="`translate(${node.x}, ${node.y})`"
              class="cursor-grab"
              @click="selectNode(node.id)"
              @dblclick.stop="handleNodeDoubleClick(node)"
              @pointerdown.stop.prevent="startDrag(node, $event)"
            >
              <title>{{ node.label }} · {{ node.typeLabel }}</title>
              <!-- Bubble shape (not color-only marker, docs/17 §7) -->
              <circle
                :r="node.r"
                :class="[
                  'stroke-2 transition-[stroke-width] duration-100',
                  node.virtual
                    ? 'stroke-orange-400 fill-orange-50'
                    : selectedNodeId === node.id
                      ? 'stroke-blue-500 fill-blue-50 stroke-[3px]'
                      : 'stroke-[var(--ui-border)] fill-[var(--ui-bg)]',
                ]"
              />
              <!-- Node icon by type (not color only) -->
              <text :y="-3" class="fill-[var(--ui-text)] text-xs font-medium pointer-events-none" text-anchor="middle">
                {{ truncateLabel(node.label) }}
              </text>
              <text :y="12" class="fill-[var(--ui-text-muted)] text-[10px] pointer-events-none" text-anchor="middle">
                {{ truncateLabel(node.typeLabel, 14) }}
              </text>
              <!-- Cycle marker (docs/17 §7) -->
              <text v-if="node.cycle" :y="node.r - 6" class="fill-orange-500 text-[10px] pointer-events-none" text-anchor="middle">
                ↩
              </text>
              <!-- Expand button for non-leaf nodes -->
              <circle
                v-if="!node.expanded && node.hasChildren"
                :cx="node.r * 0.72"
                :cy="-node.r * 0.72"
                r="9"
                class="fill-blue-500 stroke-white cursor-pointer"
                @click.stop="handleNodeDoubleClick(node)"
              />
              <text
                v-if="!node.expanded && node.hasChildren"
                :x="node.r * 0.72"
                :y="-node.r * 0.72 + 4"
                class="fill-white text-xs pointer-events-none"
                text-anchor="middle"
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
    <p v-if="!showTree" class="text-xs text-[var(--ui-text-muted)]">
      {{ t('entity.graph.hint', { nodes: nodes.length, edges: edges.length }) }}
    </p>

    <!-- Tree view (accessible alternative, docs/17 §7, docs/16 §12) -->
    <div v-if="showTree" class="rounded-lg border border-[var(--ui-border)] p-4">
      <ul role="tree" :aria-label="t('entity.graph.aria_label')">
        <GraphTreeNode
          v-for="node in treeRoots"
          :key="node.id"
          :node="node"
          :all-nodes="nodeMap"
          :edges="visibleGraph.edges"
          :visited="visited"
          :selected-node-id="selectedNodeId"
          :ancestors="[]"
          :depth="0"
          @select="selectNode"
          @expand="expandTreeNode"
        />
      </ul>
    </div>

    <!-- Context panel (docs/17 §7: selection opens context panel) -->
    <UCard v-if="selectedNode">
      <template #header>
        <h3 class="font-medium">{{ selectedNode.label }}</h3>
        <p class="text-sm text-[var(--ui-text-muted)]">{{ selectedNode.typeLabel }}</p>
      </template>
      <dl class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-sm">
        <dt class="text-[var(--ui-text-muted)]">ID</dt>
        <dd>{{ selectedNode.id }}</dd>
        <dt class="text-[var(--ui-text-muted)]">{{ t('entity.graph.edges') }}</dt>
        <dd>{{ getEdgesForNode(selectedNode.id).length }}</dd>
      </dl>
      <template #footer>
        <UButton v-if="!selectedNode.virtual" size="sm" @click="navigateToEntity(selectedNode.id)">
          {{ t('entity.open') }}
        </UButton>
        <UButton v-else size="sm" variant="outline" @click="expandCluster(selectedNode)">
          {{ t('entity.graph.expand') }}
        </UButton>
      </template>
    </UCard>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  forceX,
  forceY,
  type Simulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
} from 'd3-force';
import GraphTreeNode from './GraphTreeNode.vue';
const props = defineProps<{
  rootEntityId: string;
  rootTypeId: string;
  maxDepth?: number;
}>();

const { t, locale } = useI18n();
const toast = useToast();
const api = useApi();

// --- State ---
const asOf = ref<string>('');
const depth = ref(1);
const direction = ref<'outgoing' | 'incoming' | 'both'>('both');
const showTree = ref(false);
const nodes = ref<GraphNode[]>([]);
const edges = ref<GraphEdge[]>([]);
const visited = ref<Set<string>>(new Set());
const expandedClusters = ref<Set<string>>(new Set());
const selectedNodeId = ref<string | null>(null);
const zoom = ref(1);
const graphContainer = ref<HTMLElement | null>(null);
const svgEl = ref<SVGSVGElement | null>(null);

const maxDepth = computed(() => props.maxDepth ?? 5);

function zoomIn() {
  zoom.value = Math.min(2.5, Number((zoom.value + 0.2).toFixed(2)));
}
function zoomOut() {
  zoom.value = Math.max(0.3, Number((zoom.value - 0.2).toFixed(2)));
}
function zoomFit() {
  const container = graphContainer.value?.parentElement;
  if (!container) {
    zoom.value = 1;
    return;
  }
  zoom.value = Math.min(2, Math.max(0.3, container.clientWidth / graphWidth.value));
}

const directionOptions = computed(() => [
  { label: t('entity.graph.direction_outgoing'), value: 'outgoing' },
  { label: t('entity.graph.direction_incoming'), value: 'incoming' },
  { label: t('entity.graph.direction_both'), value: 'both' },
]);

// --- Types ---
interface GraphNode extends SimulationNodeDatum {
  id: string;
  label: string;
  typeLabel: string;
  typeId: string;
  expanded: boolean;
  hasChildren: boolean;
  cycle: boolean;
  r: number;
  status?: string;
  virtual?: boolean;
  clusterKey?: string;
  memberNodeIds?: string[];
}

interface GraphEdge {
  id: string;
  source: string;
  target: string;
  predicate: string;
  label?: string;
  cycle: boolean;
  virtual?: boolean;
}

// --- Layout constants ---
const clusterThreshold = 20;
const graphPadding = 140;
const levelGap = 250;

// --- Force simulation state ---
// Latest rendered position per node id. Mutated by the sim tick; `tick` is
// the reactivity trigger for the computed views below.
const positions = new Map<string, { x: number; y: number }>();
const tick = ref(0);
// BFS level from the root (outgoing +1, incoming -1) — feeds a weak
// directional forceX so the graph keeps a left→right reading order without
// rigid columns.
const levelById = new Map<string, number>();
// Nodes the user has manually placed stay pinned across reheats.
const pinned = new Set<string>();
let sim: Simulation<GraphNode, SimulationLinkDatum<GraphNode>> | null = null;
let seedCounter = 0;

function posOf(id: string): { x: number; y: number } {
  return positions.get(id) ?? { x: 0, y: 0 };
}

function radiusFor(id: string, deg: number): number {
  const base = id === props.rootEntityId ? 40 : 34;
  return base + Math.min(deg, 12) * 1.5;
}

const degreeById = computed(() => {
  const map = new Map<string, number>();
  for (const e of visibleGraph.value.edges) {
    map.set(e.source, (map.get(e.source) ?? 0) + 1);
    map.set(e.target, (map.get(e.target) ?? 0) + 1);
  }
  return map;
});

const visibleGraph = computed(() => {
  const expandableAnchors = new Set([props.rootEntityId, ...visited.value]);
  const grouped = new Map<string, GraphEdge[]>();
  for (const edge of edges.value) {
    const anchorId = expandableAnchors.has(edge.source)
      ? edge.source
      : expandableAnchors.has(edge.target)
        ? edge.target
        : edge.source;
    const key = `${anchorId}:${edge.predicate}:${anchorId === edge.source ? 'outgoing' : 'incoming'}`;
    const group = grouped.get(key) ?? [];
    group.push(edge);
    grouped.set(key, group);
  }

  const hiddenNodeIds = new Set<string>();
  const hiddenEdgeIds = new Set<string>();
  const virtualNodes: GraphNode[] = [];
  const virtualEdges: GraphEdge[] = [];
  for (const [clusterKey, group] of grouped) {
    if (group.length <= clusterThreshold || expandedClusters.value.has(clusterKey)) continue;
    const first = group[0];
    if (!first) continue;
    const [anchorId] = clusterKey.split(':');
    const anchor = nodes.value.find((node) => node.id === anchorId);
    if (!anchor) continue;
    const outgoing = first.source === anchor.id;
    const memberNodeIds = group.map((edge) => outgoing ? edge.target : edge.source);
    const clusterId = `cluster:${clusterKey}`;
    for (const edge of group) {
      hiddenEdgeIds.add(edge.id);
    }
    const anchorPos = posOf(anchor.id);
    virtualNodes.push({
      id: clusterId,
      label: `${first.label ?? first.predicate} · ${group.length}`,
      typeLabel: t('entity.graph.grouped_relations', { count: group.length }),
      typeId: 'relation_cluster',
      expanded: false,
      hasChildren: true,
      cycle: false,
      x: anchorPos.x + (outgoing ? levelGap : -levelGap),
      y: anchorPos.y,
      r: 40,
      virtual: true,
      clusterKey,
      memberNodeIds,
    });
    // Cluster floats one level further in the edge's direction.
    if (!levelById.has(clusterId)) {
      levelById.set(
        clusterId,
        (levelById.get(anchor.id) ?? 0) + (outgoing ? 1 : -1),
      );
    }
    virtualEdges.push({
      id: `virtual:${clusterKey}`,
      source: outgoing ? anchor.id : clusterId,
      target: outgoing ? clusterId : anchor.id,
      predicate: first.predicate,
      label: `${group.length}`,
      cycle: false,
      virtual: true,
    });
  }

  // Do not hide a grouped member if another visible relation still points to it.
  for (const node of nodes.value) {
    if (node.id === props.rootEntityId) continue;
    const incident = edges.value.filter((edge) => edge.source === node.id || edge.target === node.id);
    if (incident.length > 0 && incident.every((edge) => hiddenEdgeIds.has(edge.id))) {
      hiddenNodeIds.add(node.id);
    }
  }

  return {
    nodes: [
      ...nodes.value.filter((node) => !hiddenNodeIds.has(node.id)),
      ...virtualNodes,
    ],
    edges: [
      ...edges.value.filter((edge) => !hiddenEdgeIds.has(edge.id)),
      ...virtualEdges,
    ],
  };
});

const layoutNodes = computed(() => {
  void tick.value;
  return visibleGraph.value.nodes.map((n) => {
    const p = positions.get(n.id);
    return {
      ...n,
      x: p?.x ?? n.x ?? 0,
      y: p?.y ?? n.y ?? 0,
      r: radiusFor(n.id, degreeById.value.get(n.id) ?? 0),
    };
  });
});

const viewExtent = computed(() => {
  void tick.value;
  let minX = -graphPadding;
  let minY = -280;
  let maxX = 800;
  let maxY = 280;
  for (const node of layoutNodes.value) {
    minX = Math.min(minX, node.x - node.r - 24);
    minY = Math.min(minY, node.y - node.r - 40);
    maxX = Math.max(maxX, node.x + node.r + 24);
    maxY = Math.max(maxY, node.y + node.r + 40);
  }
  return { minX, minY, width: maxX - minX, height: maxY - minY };
});
/// On dense graphs edge labels pile up faster than any offset can fix — hide
/// them (edge titles still work on hover); virtual-edge counts stay visible.
const denseGraph = computed(() => visibleGraph.value.edges.length > 30);

const viewMinX = computed(() => viewExtent.value.minX);
const viewMinY = computed(() => viewExtent.value.minY);
const graphWidth = computed(() => Math.max(1000, viewExtent.value.width));
const graphHeight = computed(() => Math.max(560, viewExtent.value.height));

const layoutEdges = computed(() => {
  void tick.value;
  const nodeMap = new Map(layoutNodes.value.map((n) => [n.id, n]));
  // Group edges by unordered endpoint pair so parallel edges fan out on
  // quadratic curves instead of drawing on top of each other.
  const pairGroups = new Map<string, number>();
  const pairIndex = new Map<string, number>();
  for (const e of visibleGraph.value.edges) {
    const key = [e.source, e.target].sort().join('|');
    pairGroups.set(key, (pairGroups.get(key) ?? 0) + 1);
  }
  return visibleGraph.value.edges.map((e) => {
    const s = nodeMap.get(e.source);
    const tg = nodeMap.get(e.target);
    const x1 = s?.x ?? 0;
    const y1 = s?.y ?? 0;
    const x2 = tg?.x ?? 0;
    const y2 = tg?.y ?? 0;
    const dx = x2 - x1;
    const dy = y2 - y1;
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len;
    const uy = dy / len;
    const key = [e.source, e.target].sort().join('|');
    const count = pairGroups.get(key) ?? 1;
    const index = pairIndex.get(key) ?? 0;
    pairIndex.set(key, index + 1);
    // Parallel edges: bow each one around the shared chord.
    const bow = count > 1 ? (index - (count - 1) / 2) * 56 : 0;
    const cx = (x1 + x2) / 2 + (-uy) * bow;
    const cy = (y1 + y2) / 2 + (ux) * bow;
    // Trim endpoints to the node boundary so arrows land on the rim.
    const sx = x1 + ux * ((s?.r ?? 34) + 3);
    const sy = y1 + uy * ((s?.r ?? 34) + 3);
    const ex = x2 - ux * ((tg?.r ?? 34) + 9);
    const ey = y2 - uy * ((tg?.r ?? 34) + 9);
    // Labels sit ~1/3 along the path from the source, not at the midpoint —
    // on hub-and-spoke graphs midpoints of different edges all converge on
    // the hub, so labels piled up there. Anchoring near the source fans them
    // out radially around each node instead.
    const t = 0.35;
    let labelX: number;
    let labelY: number;
    if (bow !== 0) {
      const mt = 1 - t;
      labelX = mt * mt * sx + 2 * mt * t * cx + t * t * ex;
      labelY = mt * mt * sy + 2 * mt * t * cy + t * t * ey - 7;
    } else {
      labelX = sx + (ex - sx) * t;
      labelY = sy + (ey - sy) * t - 7;
    }
    return {
      ...e,
      x1: sx,
      y1: sy,
      x2: ex,
      y2: ey,
      d: `M ${sx} ${sy} Q ${cx} ${cy} ${ex} ${ey}`,
      labelX,
      labelY,
    };
  });
});

// --- Force simulation ---

/// Deterministic seed ring so the first render is not a pile at the origin.
function seedNode(node: GraphNode, anchor?: { x: number; y: number }) {
  const level = levelById.get(node.id) ?? 0;
  const base = anchor ?? { x: level * levelGap, y: 0 };
  const idx = seedCounter++;
  const angle = idx * 2.399963; // golden angle — spreads seeds evenly
  const radius = 70 + (idx % 9) * 26;
  node.x = base.x + Math.cos(angle) * radius;
  node.y = base.y + Math.sin(angle) * radius;
}

function linkDistance(l: SimulationLinkDatum<GraphNode>): number {
  const s = l.source as GraphNode;
  const tg = l.target as GraphNode;
  return (s?.r ?? 34) + (tg?.r ?? 34) + 80;
}

function onSimTick() {
  if (!sim) return;
  for (const n of sim.nodes()) {
    positions.set(n.id, { x: n.x ?? 0, y: n.y ?? 0 });
  }
  tick.value++;
}

/// (Re)build the simulation over the currently visible nodes.
/// `pinExisting` keeps every node that already has a position exactly where
/// it is — used when a neighbourhood is added so the graph does not reshuffle
/// under the user's eyes; only the newcomers float into place.
function rebuildSim(alpha: number, pinExisting: boolean) {
  sim?.stop();
  const simNodes = visibleGraph.value.nodes;
  for (const n of simNodes) {
    n.r = radiusFor(n.id, degreeById.value.get(n.id) ?? 0);
    const p = positions.get(n.id);
    const needsSeed = n.x === undefined || n.y === undefined || (n.x === 0 && n.y === 0);
    if (p) {
      n.x = p.x;
      n.y = p.y;
    } else if (needsSeed) {
      seedNode(n);
    }
    if ((pinExisting && p) || pinned.has(n.id)) {
      n.fx = n.x;
      n.fy = n.y;
    } else {
      n.fx = undefined;
      n.fy = undefined;
    }
  }
  // The root stays anchored at the level origin.
  const root = simNodes.find((n) => n.id === props.rootEntityId);
  if (root) {
    root.fx = root.fx ?? 0;
    root.fy = root.fy ?? 0;
    positions.set(root.id, { x: root.fx, y: root.fy });
  }
  const links: SimulationLinkDatum<GraphNode>[] = visibleGraph.value.edges.map((e) => ({
    source: e.source,
    target: e.target,
  }));
  sim = forceSimulation<GraphNode, SimulationLinkDatum<GraphNode>>(simNodes)
    .force(
      'link',
      forceLink<GraphNode, SimulationLinkDatum<GraphNode>>(links)
        .id((d) => d.id)
        .distance(linkDistance)
        .strength(0.7),
    )
    .force('charge', forceManyBody<GraphNode>().strength((d) => -300 - (d.r ?? 34) * 9))
    .force('collide', forceCollide<GraphNode>().radius((d) => (d.r ?? 34) + 14).strength(0.9))
    .force(
      'x',
      forceX<GraphNode>().x((d) => (levelById.get(d.id) ?? 0) * levelGap).strength(0.07),
    )
    .force('y', forceY<GraphNode>(0).strength(0.06))
    .alpha(alpha)
    .alphaDecay(0.045)
    .on('tick', onSimTick);
  onSimTick();
}

// --- Drag ---

const dragId = ref<string | null>(null);

function liveSimNode(id: string): GraphNode | undefined {
  return sim?.nodes().find((n) => n.id === id);
}

function svgPoint(evt: PointerEvent): { x: number; y: number } {
  const svg = svgEl.value;
  if (!svg) return { x: 0, y: 0 };
  const pt = svg.createSVGPoint();
  pt.x = evt.clientX;
  pt.y = evt.clientY;
  const ctm = svg.getScreenCTM();
  if (!ctm) return { x: pt.x, y: pt.y };
  const p = pt.matrixTransform(ctm.inverse());
  return { x: p.x, y: p.y };
}

function startDrag(node: GraphNode, _evt: PointerEvent) {
  dragId.value = node.id;
  const live = liveSimNode(node.id);
  if (live) {
    const p = posOf(node.id);
    live.fx = p.x;
    live.fy = p.y;
    sim?.alphaTarget(0.2).restart();
  }
}

function onDragMove(evt: PointerEvent) {
  if (!dragId.value) return;
  const live = liveSimNode(dragId.value);
  if (!live) return;
  const p = svgPoint(evt);
  live.fx = p.x;
  live.fy = p.y;
}

function endDrag() {
  if (!dragId.value) return;
  const live = liveSimNode(dragId.value);
  if (live) {
    // Respect the drop position — the node stays pinned where the user left it.
    pinned.add(dragId.value);
    positions.set(dragId.value, { x: live.fx ?? 0, y: live.fy ?? 0 });
  }
  dragId.value = null;
  sim?.alphaTarget(0);
}

// --- Tree view ---
const nodeMap = computed(() => {
  const map = new Map<string, GraphNode>();
  layoutNodes.value.forEach((n) => map.set(n.id, n));
  return map;
});

const treeRoots = computed(() => {
  const root = nodeMap.value.get(props.rootEntityId);
  if (root) return [root];
  const childIds = new Set(visibleGraph.value.edges.map((e) => e.target));
  return layoutNodes.value.filter((n) => !childIds.has(n.id));
});

// --- Selected node ---
const selectedNode = computed(() =>
  selectedNodeId.value ? nodeMap.value.get(selectedNodeId.value) ?? null : null,
);

function getEdgesForNode(nodeId: string): GraphEdge[] {
  return visibleGraph.value.edges.filter((e) => e.source === nodeId || e.target === nodeId);
}

// --- Actions ---
function selectNode(id: string) {
  selectedNodeId.value = id;
}

function navigateToEntity(id: string) {
  navigateTo(`/entities/${id}`);
}

function truncateLabel(label: string, max = 16): string {
  return label.length > max ? `${label.slice(0, max - 1)}…` : label;
}

/// BFS levels from the root: outgoing edge → +1, incoming → -1.
function computeLevels() {
  levelById.clear();
  levelById.set(props.rootEntityId, 0);
  const queue = [props.rootEntityId];
  while (queue.length > 0) {
    const current = queue.shift()!;
    const level = levelById.get(current) ?? 0;
    for (const edge of edges.value) {
      const outgoing = edge.source === current;
      const incoming = edge.target === current;
      const neighbour = outgoing ? edge.target : incoming ? edge.source : undefined;
      if (neighbour && !levelById.has(neighbour)) {
        levelById.set(neighbour, level + (outgoing ? 1 : -1));
        queue.push(neighbour);
      }
    }
  }
}

/// Seed freshly added nodes on a small ring around the anchor they joined
/// through, then let the simulation (existing nodes pinned) float them into
/// collision-free positions.
function seedAddedNodes(anchorId: string, nodeIds: string[]) {
  if (nodeIds.length === 0) return;
  const anchorPos = posOf(anchorId);
  const anchorLevel = levelById.get(anchorId) ?? 0;
  for (const id of nodeIds) {
    const node = nodes.value.find((n) => n.id === id);
    if (!node) continue;
    const joining = edges.value.find(
      (e) => (e.source === anchorId && e.target === id) || (e.target === anchorId && e.source === id),
    );
    const dir = joining?.target === anchorId ? -1 : 1;
    if (!levelById.has(id)) levelById.set(id, anchorLevel + dir);
    // Seed position lives only on the node object; it must NOT enter
    // `positions`, or rebuildSim would pin the seed as-is and the bubble
    // could never float to a collision-free spot.
    seedNode(node, { x: anchorPos.x + dir * 60, y: anchorPos.y });
  }
}

function expandCluster(node: GraphNode) {
  if (!node.clusterKey) return;
  const clusterPos = posOf(node.id);
  expandedClusters.value = new Set([...expandedClusters.value, node.clusterKey]);
  // Seed the revealed members around the collapsed cluster position.
  for (const memberId of node.memberNodeIds ?? []) {
    const member = nodes.value.find((n) => n.id === memberId);
    if (member && !positions.has(memberId)) {
      seedNode(member, clusterPos);
    }
  }
  selectedNodeId.value = node.memberNodeIds?.[0] ?? null;
  rebuildSim(0.5, true);
}

/// Double-click on a virtual edge: expand the underlying relation cluster so
/// the hidden members materialize onto the canvas.
function handleVirtualEdgeClick(edge: GraphEdge) {
  const clusterId = edge.virtual ? (edge.source.startsWith('cluster:') ? edge.source : edge.target) : '';
  const cluster = layoutNodes.value.find((n) => n.id === clusterId);
  if (cluster) {
    expandCluster(cluster);
    return;
  }
  // Fallback: resolve the cluster node by its virtual edge id.
  const key = edge.id.replace(/^virtual:/, 'cluster:');
  const fallback = layoutNodes.value.find((n) => n.id === key);
  if (fallback) expandCluster(fallback);
}

function selectVirtualEdge(edge: GraphEdge) {
  const clusterId = edge.source.startsWith('cluster:') ? edge.source : edge.target;
  if (clusterId.startsWith('cluster:')) selectNode(clusterId);
}

function handleNodeDoubleClick(node: GraphNode) {
  if (node.virtual) {
    expandCluster(node);
    return;
  }
  expandNode(node.id);
}

function expandTreeNode(nodeId: string) {
  const node = nodeMap.value.get(nodeId);
  if (node) handleNodeDoubleClick(node);
}

async function expandNode(nodeId: string) {
  if (visited.value.has(nodeId)) return;
  visited.value.add(nodeId);

  // Fetch relations for this node (progressive traversal, docs/17 §7)
  try {
    const res = await api.entities.getGraph(nodeId, {
      max_depth: depth.value,
      direction: direction.value,
      as_of: asOf.value || undefined,
      locale: locale.value,
    });
    const apiNodes = res.data?.nodes ?? [];
    const apiEdges = res.data?.edges ?? [];
    const addedNodeIds: string[] = [];
    // Add new nodes (skip duplicates)
    for (const n of apiNodes) {
      if (!nodes.value.some((existing) => existing.id === n.id)) {
        addedNodeIds.push(n.id);
        nodes.value.push({
          id: n.id,
          label: n.label,
          typeLabel: n.type_label,
          typeId: n.type_id,
          expanded: false,
          hasChildren: n.has_children ?? false,
          cycle: n.cycle ?? false,
          x: 0,
          y: 0,
          r: 34,
          status: n.status,
        });
      }
    }
    // Add new edges (skip duplicates)
    for (const e of apiEdges) {
      const edgeId = e.id ?? `${e.source}:${e.predicate ?? e.label ?? ''}:${e.target}`;
      if (!edges.value.some((existing) => existing.id === edgeId)) {
        edges.value.push({
          id: edgeId,
          source: e.source,
          target: e.target,
          predicate: e.predicate ?? e.label ?? '',
          label: e.label,
          cycle: e.cycle ?? false,
        });
      }
    }
    seedAddedNodes(nodeId, addedNodeIds);
    rebuildSim(0.6, true);
    const node = nodes.value.find((n) => n.id === nodeId);
    if (node) {
      node.expanded = true;
    }
  } catch (e: unknown) {
    visited.value = new Set([...visited.value].filter((id) => id !== nodeId));
    toast.add({
      title: t('common.error'),
      description: e instanceof Error ? e.message : String(e),
      color: 'error',
    });
  }
}

async function reload() {
  sim?.stop();
  nodes.value = [];
  edges.value = [];
  visited.value.clear();
  expandedClusters.value = new Set();
  positions.clear();
  pinned.clear();
  levelById.clear();
  seedCounter = 0;
  try {
    const res = await api.entities.getGraph(props.rootEntityId, {
      max_depth: depth.value,
      direction: direction.value,
      as_of: asOf.value || undefined,
      locale: locale.value,
    });
    const apiNodes = res.data?.nodes ?? [];
    const apiEdges = res.data?.edges ?? [];
    nodes.value = apiNodes.map((n) => ({
      id: n.id,
      label: n.label,
      typeLabel: n.type_label,
      typeId: n.type_id,
      expanded: false,
      hasChildren: n.has_children ?? false,
      cycle: n.cycle ?? false,
      x: 0,
      y: 0,
      r: 34,
      status: n.status,
    }));
    edges.value = apiEdges.map((e) => ({
      id: e.id ?? `${e.source}:${e.predicate ?? e.label ?? ''}:${e.target}`,
      source: e.source,
      target: e.target,
      predicate: e.predicate ?? e.label ?? '',
      label: e.label,
      cycle: e.cycle ?? false,
    }));
    computeLevels();
    // Mark root as visited so we don't re-fetch it
    visited.value.add(props.rootEntityId);
    const rootNode = nodes.value.find((n) => n.id === props.rootEntityId);
    if (rootNode) {
      rootNode.expanded = true;
    }
    rebuildSim(1, false);
  } catch (e: unknown) {
    toast.add({ title: t('common.error'), description: e instanceof Error ? e.message : String(e), color: 'error' });
  }
}

onMounted(reload);
onBeforeUnmount(() => sim?.stop());
watch(() => props.rootEntityId, reload);
</script>
