import { create } from "zustand";
import {
  channelList,
  channelListTypes,
  channelConnect,
  channelDisconnect,
  channelLogin,
  channelSend,
  channelCreateInstance,
  channelRemoveInstance,
  bindingList,
  bindingAdd,
  bindingRemove,
  type ChannelDto,
  type ChannelTypeDto,
  type BindingDto,
} from "../lib/tauri-commands";

interface ChannelState {
  channels: ChannelDto[];
  channelTypes: ChannelTypeDto[];
  channelBindings: Record<string, BindingDto[]>;
  loading: boolean;

  loadChannels: () => Promise<void>;
  loadChannelTypes: () => Promise<void>;
  loadBindings: () => Promise<void>;
  loadAll: () => Promise<void>;

  createInstance: (
    channelType: string,
    instanceId: string,
    displayName?: string,
  ) => Promise<string>;
  removeInstance: (channelType: string, instanceId: string) => Promise<void>;
  connect: (
    channelId: string,
    settings: Record<string, unknown>,
  ) => Promise<void>;
  disconnect: (channelId: string) => Promise<void>;
  login: (
    channelId: string,
    credentialType: string,
    data: Record<string, string>,
  ) => Promise<string>;
  send: (
    channelId: string,
    recipient: string,
    text: string,
  ) => Promise<void>;

  addBindingForChannel: (
    channelInstance: string,
    extensionId: string,
    peerFilter?: string,
    groupFilter?: string,
    priority?: number,
  ) => Promise<string>;
  removeBindingForChannel: (bindingId: string) => Promise<void>;
  getBindingsForChannel: (channelId: string) => BindingDto[];
}

function groupBindings(bindings: BindingDto[]): Record<string, BindingDto[]> {
  const groups: Record<string, BindingDto[]> = {};
  for (const b of bindings) {
    const key = b.channel_instance;
    if (!groups[key]) groups[key] = [];
    groups[key].push(b);
  }
  return groups;
}

export const useChannelStore = create<ChannelState>((set, get) => ({
  channels: [],
  channelTypes: [],
  channelBindings: {},
  loading: false,

  loadChannels: async () => {
    set({ loading: true });
    try {
      const channels = await channelList();
      set({ channels, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  loadChannelTypes: async () => {
    try {
      const channelTypes = await channelListTypes();
      set({ channelTypes });
    } catch {
      // ignore
    }
  },

  loadBindings: async () => {
    try {
      const bindings = await bindingList();
      set({ channelBindings: groupBindings(bindings) });
    } catch {
      // ignore
    }
  },

  loadAll: async () => {
    set({ loading: true });
    try {
      const [channels, channelTypes, bindings] = await Promise.all([
        channelList(),
        channelListTypes(),
        bindingList(),
      ]);
      set({
        channels,
        channelTypes,
        channelBindings: groupBindings(bindings),
        loading: false,
      });
    } catch {
      set({ loading: false });
    }
  },

  createInstance: async (channelType, instanceId, displayName) => {
    const key = await channelCreateInstance(channelType, instanceId, displayName);
    const channels = await channelList();
    set({ channels });
    return key;
  },

  removeInstance: async (channelType, instanceId) => {
    await channelRemoveInstance(channelType, instanceId);
    const [channels, bindings] = await Promise.all([channelList(), bindingList()]);
    set({ channels, channelBindings: groupBindings(bindings) });
  },

  connect: async (channelId, settings) => {
    await channelConnect(channelId, settings);
    const channels = await channelList();
    set({ channels });
  },

  disconnect: async (channelId) => {
    await channelDisconnect(channelId);
    const channels = await channelList();
    set({ channels });
  },

  login: async (channelId, credentialType, data) => {
    const status = await channelLogin(channelId, credentialType, data);
    const channels = await channelList();
    set({ channels });
    return status;
  },

  send: async (channelId, recipient, text) => {
    await channelSend(channelId, recipient, text);
  },

  addBindingForChannel: async (
    channelInstance,
    extensionId,
    peerFilter,
    groupFilter,
    priority,
  ) => {
    const id = await bindingAdd(channelInstance, extensionId, peerFilter, groupFilter, priority);
    const bindings = await bindingList();
    set({ channelBindings: groupBindings(bindings) });
    return id;
  },

  removeBindingForChannel: async (bindingId) => {
    await bindingRemove(bindingId);
    const bindings = await bindingList();
    set({ channelBindings: groupBindings(bindings) });
  },

  getBindingsForChannel: (channelId: string) => {
    return get().channelBindings[channelId] ?? [];
  },
}));
