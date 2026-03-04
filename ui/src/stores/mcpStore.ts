import { create } from "zustand";
import {
  mcpListServers,
  mcpAddServer,
  mcpRemoveServer,
  mcpUpdateServer,
  mcpStartServer,
  mcpStopServer,
  mcpRestartServer,
  mcpGetServerTools,
  type McpServerDto,
  type McpToolDto,
} from "../lib/tauri-commands";

interface McpState {
  servers: McpServerDto[];
  loading: boolean;
  selectedServerName: string | null;
  serverTools: McpToolDto[];
  error: string | null;
  actionInProgress: string | null;

  loadServers: () => Promise<void>;
  selectServer: (name: string | null) => void;

  addServer: (
    name: string,
    command: string,
    args: string[],
    env: Record<string, string>,
    workingDir?: string,
    autoStart?: boolean,
    connectNow?: boolean,
  ) => Promise<void>;

  removeServer: (name: string) => Promise<void>;

  updateServer: (
    name: string,
    command?: string,
    args?: string[],
    env?: Record<string, string>,
    workingDir?: string,
    autoStart?: boolean,
  ) => Promise<void>;

  startServer: (name: string) => Promise<void>;
  stopServer: (name: string) => Promise<void>;
  restartServer: (name: string) => Promise<void>;
  loadServerTools: (name: string) => Promise<void>;
}

export const useMcpStore = create<McpState>((set, get) => ({
  servers: [],
  loading: false,
  selectedServerName: null,
  serverTools: [],
  error: null,
  actionInProgress: null,

  loadServers: async () => {
    set({ loading: true });
    try {
      const servers = await mcpListServers();
      set({ servers, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  selectServer: (name) => {
    set({ selectedServerName: name, serverTools: [], error: null });
    if (name) {
      const server = get().servers.find((s) => s.name === name);
      if (server && server.status === "connected") {
        get().loadServerTools(name);
      }
    }
  },

  addServer: async (name, command, args, env, workingDir, autoStart, connectNow) => {
    set({ error: null });
    try {
      await mcpAddServer(name, command, args, env, workingDir, autoStart, connectNow);
      const servers = await mcpListServers();
      set({ servers, selectedServerName: name });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  removeServer: async (name) => {
    set({ error: null });
    try {
      await mcpRemoveServer(name);
      const servers = await mcpListServers();
      const selected =
        get().selectedServerName === name ? null : get().selectedServerName;
      set({ servers, selectedServerName: selected, serverTools: selected ? get().serverTools : [] });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateServer: async (name, command, args, env, workingDir, autoStart) => {
    set({ error: null });
    try {
      await mcpUpdateServer(name, command, args, env, workingDir, autoStart);
      const servers = await mcpListServers();
      set({ servers });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  startServer: async (name) => {
    set({ actionInProgress: "starting", error: null });
    try {
      await mcpStartServer(name);
      const servers = await mcpListServers();
      set({ servers, actionInProgress: null });
      // Auto-load tools after connecting
      get().loadServerTools(name);
    } catch (e) {
      set({ actionInProgress: null, error: String(e) });
      throw e;
    }
  },

  stopServer: async (name) => {
    set({ actionInProgress: "stopping", error: null });
    try {
      await mcpStopServer(name);
      const servers = await mcpListServers();
      set({ servers, actionInProgress: null, serverTools: [] });
    } catch (e) {
      set({ actionInProgress: null, error: String(e) });
      throw e;
    }
  },

  restartServer: async (name) => {
    set({ actionInProgress: "restarting", error: null });
    try {
      await mcpRestartServer(name);
      const servers = await mcpListServers();
      set({ servers, actionInProgress: null });
      // Reload tools after restart
      get().loadServerTools(name);
    } catch (e) {
      set({ actionInProgress: null, error: String(e) });
      throw e;
    }
  },

  loadServerTools: async (name) => {
    try {
      const serverTools = await mcpGetServerTools(name);
      set({ serverTools });
    } catch {
      set({ serverTools: [] });
    }
  },
}));
