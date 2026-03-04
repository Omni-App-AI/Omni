import { create } from "zustand";
import {
  listExtensions,
  installExtension,
  activateExtension,
  deactivateExtension,
  uninstallExtension,
  toggleExtensionEnabled,
  type ExtensionDto,
} from "../lib/tauri-commands";

interface ExtensionState {
  extensions: ExtensionDto[];
  loading: boolean;

  loadExtensions: () => Promise<void>;
  install: (sourcePath: string) => Promise<string>;
  activate: (extensionId: string) => Promise<void>;
  deactivate: (extensionId: string) => Promise<void>;
  uninstall: (extensionId: string) => Promise<void>;
  toggleEnabled: (extensionId: string, enabled: boolean) => Promise<void>;
}

export const useExtensionStore = create<ExtensionState>((set) => ({
  extensions: [],
  loading: false,

  loadExtensions: async () => {
    set({ loading: true });
    const extensions = await listExtensions();
    set({ extensions, loading: false });
  },

  install: async (sourcePath: string) => {
    const id = await installExtension(sourcePath);
    const extensions = await listExtensions();
    set({ extensions });
    return id;
  },

  activate: async (extensionId: string) => {
    await activateExtension(extensionId);
    const extensions = await listExtensions();
    set({ extensions });
  },

  deactivate: async (extensionId: string) => {
    await deactivateExtension(extensionId);
    const extensions = await listExtensions();
    set({ extensions });
  },

  uninstall: async (extensionId: string) => {
    await uninstallExtension(extensionId);
    const extensions = await listExtensions();
    set({ extensions });
  },

  toggleEnabled: async (extensionId: string, enabled: boolean) => {
    await toggleExtensionEnabled(extensionId, enabled);
    const extensions = await listExtensions();
    set({ extensions });
  },
}));
