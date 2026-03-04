import { create } from "zustand";
import {
  listExtensionInstances,
  createExtensionInstance,
  deleteExtensionInstance,
  activateExtensionInstance,
  deactivateExtensionInstance,
  updateExtensionInstance,
  toggleInstanceEnabled,
  type ExtensionInstanceDto,
} from "../lib/tauri-commands";

interface InstanceState {
  /** Instances grouped by extension_id */
  instances: Record<string, ExtensionInstanceDto[]>;
  loading: boolean;

  loadInstances: (extensionId?: string) => Promise<void>;
  createInstance: (
    extensionId: string,
    instanceName: string,
    displayName?: string,
  ) => Promise<string>;
  deleteInstance: (instanceId: string) => Promise<void>;
  activateInstance: (instanceId: string) => Promise<void>;
  deactivateInstance: (instanceId: string) => Promise<void>;
  updateInstance: (instanceId: string, displayName?: string) => Promise<void>;
  toggleEnabled: (instanceId: string, enabled: boolean) => Promise<void>;
}

function groupByExtension(
  list: ExtensionInstanceDto[],
): Record<string, ExtensionInstanceDto[]> {
  const groups: Record<string, ExtensionInstanceDto[]> = {};
  for (const inst of list) {
    const key = inst.extension_id;
    if (!groups[key]) groups[key] = [];
    groups[key].push(inst);
  }
  return groups;
}

export const useInstanceStore = create<InstanceState>((set) => ({
  instances: {},
  loading: false,

  loadInstances: async (extensionId?: string) => {
    set({ loading: true });
    try {
      const list = await listExtensionInstances(extensionId);
      set({ instances: groupByExtension(list), loading: false });
    } catch {
      set({ loading: false });
    }
  },

  createInstance: async (
    extensionId: string,
    instanceName: string,
    displayName?: string,
  ) => {
    const instanceId = await createExtensionInstance(
      extensionId,
      instanceName,
      displayName,
    );
    const list = await listExtensionInstances();
    set({ instances: groupByExtension(list) });
    return instanceId;
  },

  deleteInstance: async (instanceId: string) => {
    await deleteExtensionInstance(instanceId);
    const list = await listExtensionInstances();
    set({ instances: groupByExtension(list) });
  },

  activateInstance: async (instanceId: string) => {
    await activateExtensionInstance(instanceId);
    const list = await listExtensionInstances();
    set({ instances: groupByExtension(list) });
  },

  deactivateInstance: async (instanceId: string) => {
    await deactivateExtensionInstance(instanceId);
    const list = await listExtensionInstances();
    set({ instances: groupByExtension(list) });
  },

  updateInstance: async (instanceId: string, displayName?: string) => {
    await updateExtensionInstance(instanceId, displayName);
    const list = await listExtensionInstances();
    set({ instances: groupByExtension(list) });
  },

  toggleEnabled: async (instanceId: string, enabled: boolean) => {
    await toggleInstanceEnabled(instanceId, enabled);
    const list = await listExtensionInstances();
    set({ instances: groupByExtension(list) });
  },
}));
