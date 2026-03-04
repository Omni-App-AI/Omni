import { create } from "zustand";
import {
  bindingAdd,
  bindingRemove,
  bindingList,
  type BindingDto,
} from "../lib/tauri-commands";

interface BindingState {
  bindings: BindingDto[];
  loading: boolean;

  loadBindings: () => Promise<void>;
  addBinding: (
    channelInstance: string,
    extensionId: string,
    peerFilter?: string,
    groupFilter?: string,
    priority?: number,
  ) => Promise<string>;
  removeBinding: (bindingId: string) => Promise<void>;
}

export const useBindingStore = create<BindingState>((set) => ({
  bindings: [],
  loading: false,

  loadBindings: async () => {
    set({ loading: true });
    try {
      const bindings = await bindingList();
      set({ bindings, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  addBinding: async (channelInstance, extensionId, peerFilter, groupFilter, priority) => {
    const id = await bindingAdd(channelInstance, extensionId, peerFilter, groupFilter, priority);
    const bindings = await bindingList();
    set({ bindings });
    return id;
  },

  removeBinding: async (bindingId) => {
    await bindingRemove(bindingId);
    const bindings = await bindingList();
    set({ bindings });
  },
}));
