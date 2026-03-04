import { create } from "zustand";
import {
  providerList,
  providerListTypes,
  providerAdd,
  providerUpdate,
  providerRemove,
  providerSetCredential,
  providerDeleteCredential,
  providerTestCredential,
  type ProviderDto,
  type ProviderTypeInfoDto,
} from "../lib/tauri-commands";

interface ProviderState {
  providers: ProviderDto[];
  providerTypes: ProviderTypeInfoDto[];
  loading: boolean;
  selectedProviderId: string | null;
  testResult: {
    providerId: string;
    message: string;
    success: boolean;
  } | null;
  testingProviderId: string | null;

  loadAll: () => Promise<void>;
  selectProvider: (id: string | null) => void;

  addProvider: (
    id: string,
    providerType: string,
    defaultModel?: string,
    endpoint?: string,
    maxTokens?: number,
    temperature?: number,
  ) => Promise<void>;

  updateProvider: (
    id: string,
    defaultModel?: string,
    endpoint?: string,
    maxTokens?: number,
    temperature?: number,
    enabled?: boolean,
  ) => Promise<void>;

  removeProvider: (id: string) => Promise<void>;

  setCredential: (
    providerId: string,
    credentialType: string,
    apiKey?: string,
    awsAccessKeyId?: string,
    awsSecretAccessKey?: string,
    awsSessionToken?: string,
    awsRegion?: string,
  ) => Promise<void>;

  deleteCredential: (providerId: string) => Promise<void>;

  testCredential: (providerId: string) => Promise<void>;
  clearTestResult: () => void;
}

export const useProviderStore = create<ProviderState>((set, get) => ({
  providers: [],
  providerTypes: [],
  loading: false,
  selectedProviderId: null,
  testResult: null,
  testingProviderId: null,

  loadAll: async () => {
    set({ loading: true });
    try {
      const [providers, providerTypes] = await Promise.all([
        providerList(),
        providerListTypes(),
      ]);
      set({ providers, providerTypes, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  selectProvider: (id) => {
    set({ selectedProviderId: id, testResult: null });
  },

  addProvider: async (
    id,
    providerType,
    defaultModel,
    endpoint,
    maxTokens,
    temperature,
  ) => {
    await providerAdd(
      id,
      providerType,
      defaultModel,
      endpoint,
      maxTokens,
      temperature,
    );
    const providers = await providerList();
    set({ providers, selectedProviderId: id });
  },

  updateProvider: async (
    id,
    defaultModel,
    endpoint,
    maxTokens,
    temperature,
    enabled,
  ) => {
    await providerUpdate(
      id,
      defaultModel,
      endpoint,
      maxTokens,
      temperature,
      enabled,
    );
    const providers = await providerList();
    set({ providers });
  },

  removeProvider: async (id) => {
    await providerRemove(id);
    const providers = await providerList();
    const selected =
      get().selectedProviderId === id ? null : get().selectedProviderId;
    set({ providers, selectedProviderId: selected });
  },

  setCredential: async (
    providerId,
    credentialType,
    apiKey,
    awsAccessKeyId,
    awsSecretAccessKey,
    awsSessionToken,
    awsRegion,
  ) => {
    await providerSetCredential(
      providerId,
      credentialType,
      apiKey,
      awsAccessKeyId,
      awsSecretAccessKey,
      awsSessionToken,
      awsRegion,
    );
    const providers = await providerList();
    set({ providers });
  },

  deleteCredential: async (providerId) => {
    await providerDeleteCredential(providerId);
    const providers = await providerList();
    set({ providers });
  },

  testCredential: async (providerId) => {
    set({ testingProviderId: providerId, testResult: null });
    try {
      const message = await providerTestCredential(providerId);
      set({
        testResult: { providerId, message, success: true },
        testingProviderId: null,
      });
    } catch (e) {
      set({
        testResult: { providerId, message: String(e), success: false },
        testingProviderId: null,
      });
    }
  },

  clearTestResult: () => set({ testResult: null }),
}));
