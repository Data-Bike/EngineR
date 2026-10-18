import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import GraphView from '../app/components/graph/GraphView.vue';

const uiStubs = {
  UFormField: { template: '<label><slot /></label>' },
  UInput: { template: '<input />' },
  USelect: { template: '<select />' },
  UButton: { template: '<button @click="$emit(\'click\')"><slot /></button>' },
  UCard: { template: '<section><slot name="header" /><slot /><slot name="footer" /></section>' },
  GraphTreeNode: { template: '<li />' },
};

function mountGraph(getGraph: ReturnType<typeof vi.fn>) {
  (globalThis as any).useApi = () => ({ entities: { getGraph } });
  return mount(GraphView, {
    props: { rootEntityId: 'root', rootTypeId: 'company' },
    global: { stubs: uiStubs },
  });
}

function nodeGroups(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll('g.nodes > g');
}

function transformFor(wrapper: ReturnType<typeof mount>, label: string) {
  return nodeGroups(wrapper).find((node) => node.text().includes(label))?.attributes('transform');
}

describe('GraphView user scenarios', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('collapses more than 20 equivalent relations and expands the virtual relation', async () => {
    const members = Array.from({ length: 21 }, (_, index) => ({
      id: `member-${index}`,
      label: `Участник ${index}`,
      type_label: 'Организация',
      type_id: 'company',
      has_children: false,
      cycle: false,
      status: 'active',
    }));
    const getGraph = vi.fn().mockResolvedValue({
      data: {
        nodes: [
          { id: 'root', label: 'Головная компания', type_label: 'Организация', type_id: 'company', has_children: true },
          ...members,
        ],
        edges: members.map((member, index) => ({
          id: `edge-${index}`,
          source: 'root',
          target: member.id,
          predicate: 'owns',
          label: 'Владеет',
          cycle: false,
        })),
      },
    });

    const wrapper = mountGraph(getGraph);
    await flushPromises();

    expect(nodeGroups(wrapper)).toHaveLength(2);
    const cluster = nodeGroups(wrapper).find((node) => node.text().includes('Владеет · 21'));
    expect(cluster).toBeDefined();
    expect(wrapper.text()).toContain('entity.graph.grouped_relations:21');

    await cluster!.trigger('dblclick');
    await wrapper.vm.$nextTick();

    expect(nodeGroups(wrapper)).toHaveLength(22);
    expect(wrapper.text()).toContain('Участник 20');
    expect(getGraph).toHaveBeenCalledTimes(1);
  });

  it('adds a double-clicked node neighborhood without moving or deleting existing nodes', async () => {
    const getGraph = vi
      .fn()
      .mockResolvedValueOnce({
        data: {
          nodes: [
            { id: 'root', label: 'Корень', type_label: 'Организация', type_id: 'company', has_children: true },
            { id: 'a', label: 'Узел A', type_label: 'Персона', type_id: 'person', has_children: true },
          ],
          edges: [{ id: 'root-a', source: 'root', target: 'a', predicate: 'managed_by', label: 'Управляет' }],
        },
      })
      .mockResolvedValueOnce({
        data: {
          nodes: [
            { id: 'a', label: 'Узел A', type_label: 'Персона', type_id: 'person', has_children: true },
            { id: 'b', label: 'Узел B', type_label: 'Организация', type_id: 'company', has_children: false },
          ],
          edges: [{ id: 'a-b', source: 'a', target: 'b', predicate: 'owns', label: 'Связан' }],
        },
      });

    const wrapper = mountGraph(getGraph);
    await flushPromises();
    const rootBefore = transformFor(wrapper, 'Корень');
    const nodeABefore = transformFor(wrapper, 'Узел A');
    const nodeA = nodeGroups(wrapper).find((node) => node.text().includes('Узел A'));
    expect(nodeA).toBeDefined();

    await nodeA!.trigger('dblclick');
    await flushPromises();

    expect(nodeGroups(wrapper)).toHaveLength(3);
    expect(transformFor(wrapper, 'Корень')).toBe(rootBefore);
    expect(transformFor(wrapper, 'Узел A')).toBe(nodeABefore);
    expect(nodeGroups(wrapper).some((node) => node.text().includes('Узел B'))).toBe(true);
    expect(getGraph).toHaveBeenNthCalledWith(2, 'a', {
      max_depth: 1,
      direction: 'both',
      as_of: undefined,
      locale: 'en',
    });
  });
});
