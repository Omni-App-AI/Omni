import { create } from "zustand";
import { EXAMPLE_FLOWCHARTS } from "../lib/example-flowcharts";
import {
  flowchartList,
  flowchartGet,
  flowchartSave,
  flowchartDelete,
  flowchartToggleEnabled,
  flowchartValidate,
  flowchartTest,
  type FlowchartDto,
  type FlowchartDefinitionDto,
  type FlowchartTestResultDto,
  type FlowchartValidationDto,
} from "../lib/tauri-commands";

interface FlowchartState {
  flowcharts: FlowchartDto[];
  activeFlowchart: FlowchartDefinitionDto | null;
  editorDirty: boolean;
  lastTestResult: FlowchartTestResultDto | null;
  lastValidation: FlowchartValidationDto | null;
  loading: boolean;
  error: string | null;

  loadFlowcharts: () => Promise<void>;
  openFlowchart: (id: string) => Promise<void>;
  createNew: () => void;
  loadExample: (exampleId: string) => void;
  updateActive: (partial: Partial<FlowchartDefinitionDto>) => void;
  save: () => Promise<void>;
  deleteFlowchart: (id: string) => Promise<void>;
  toggleEnabled: (id: string, enabled: boolean) => Promise<void>;
  validate: () => Promise<FlowchartValidationDto | null>;
  test: (toolName: string, params: Record<string, unknown>) => Promise<void>;
  closeEditor: () => void;
}

function makeEmptyDefinition(): FlowchartDefinitionDto {
  const now = new Date().toISOString();
  return {
    id: `flow.user.new_${Date.now()}`,
    name: "New Flowchart",
    version: "0.1.0",
    author: "User",
    description: "",
    enabled: true,
    tools: [
      {
        name: "main",
        description: "Main tool",
        parameters: { type: "object", properties: {}, required: [] },
        trigger_node_id: "trigger_1",
      },
    ],
    permissions: [],
    config: {},
    nodes: [
      {
        id: "trigger_1",
        node_type: "trigger",
        label: "Start",
        position: { x: 250, y: 50 },
        config: {},
      },
      {
        id: "output_1",
        node_type: "output",
        label: "Return",
        position: { x: 250, y: 400 },
        config: { result_template: "{{$.params}}" },
      },
    ],
    edges: [
      {
        id: "e_trigger_output",
        source: "trigger_1",
        target: "output_1",
        source_handle: null,
        target_handle: null,
        label: null,
      },
    ],
    viewport: { x: 0, y: 0, zoom: 1 },
    created_at: now,
    updated_at: now,
  };
}

export const useFlowchartStore = create<FlowchartState>((set, get) => ({
  flowcharts: [],
  activeFlowchart: null,
  editorDirty: false,
  lastTestResult: null,
  lastValidation: null,
  loading: false,
  error: null,

  loadFlowcharts: async () => {
    set({ loading: true, error: null });
    try {
      const list = await flowchartList();
      set({ flowcharts: list, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  openFlowchart: async (id: string) => {
    if (get().editorDirty) {
      if (!window.confirm("You have unsaved changes. Discard them?")) return;
    }
    set({ loading: true, error: null });
    try {
      const def = await flowchartGet(id);
      set({
        activeFlowchart: def,
        editorDirty: false,
        lastTestResult: null,
        lastValidation: null,
        loading: false,
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createNew: () => {
    set({
      activeFlowchart: makeEmptyDefinition(),
      editorDirty: true,
      lastTestResult: null,
      lastValidation: null,
    });
  },

  loadExample: (exampleId: string) => {
    const example = EXAMPLE_FLOWCHARTS.find((e) => e.id === exampleId);
    if (!example) return;
    set({
      activeFlowchart: example.create(),
      editorDirty: true,
      lastTestResult: null,
      lastValidation: null,
    });
  },

  updateActive: (partial) => {
    const active = get().activeFlowchart;
    if (!active) return;
    set({
      activeFlowchart: { ...active, ...partial },
      editorDirty: true,
    });
  },

  save: async () => {
    const active = get().activeFlowchart;
    if (!active) return;
    set({ loading: true, error: null });
    try {
      await flowchartSave(active);
      set({ editorDirty: false, loading: false });
      await get().loadFlowcharts();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  deleteFlowchart: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await flowchartDelete(id);
      const active = get().activeFlowchart;
      if (active?.id === id) {
        set({ activeFlowchart: null, editorDirty: false });
      }
      set({ loading: false });
      await get().loadFlowcharts();
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  toggleEnabled: async (id: string, enabled: boolean) => {
    try {
      await flowchartToggleEnabled(id, enabled);
      await get().loadFlowcharts();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  validate: async () => {
    const active = get().activeFlowchart;
    if (!active) return null;
    try {
      const result = await flowchartValidate(active);
      set({ lastValidation: result });
      return result;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  test: async (toolName: string, params: Record<string, unknown>) => {
    const active = get().activeFlowchart;
    if (!active) return;
    set({ loading: true, error: null });
    try {
      const result = await flowchartTest(active.id, toolName, params);
      set({ lastTestResult: result, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  closeEditor: () => {
    if (get().editorDirty) {
      if (!window.confirm("You have unsaved changes. Discard them?")) return;
    }
    set({
      activeFlowchart: null,
      editorDirty: false,
      lastTestResult: null,
      lastValidation: null,
    });
  },
}));
