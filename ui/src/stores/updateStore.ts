import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

interface UpdateState {
  status: UpdateStatus;
  update: Update | null;
  version: string | null;
  notes: string | null;
  progress: number;
  error: string | null;
  dismissed: boolean;
  lastChecked: number | null;

  checkForUpdate: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  dismiss: () => void;
}

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000; // 6 hours

export const useUpdateStore = create<UpdateState>((set, get) => ({
  status: "idle",
  update: null,
  version: null,
  notes: null,
  progress: 0,
  error: null,
  dismissed: false,
  lastChecked: null,

  checkForUpdate: async () => {
    const { status } = get();
    if (status === "checking" || status === "downloading") return;

    set({ status: "checking", error: null });

    try {
      const update = await check();
      if (update) {
        set({
          status: "available",
          update,
          version: update.version,
          notes: update.body ?? null,
          dismissed: false,
          lastChecked: Date.now(),
        });
      } else {
        set({
          status: "idle",
          update: null,
          version: null,
          notes: null,
          lastChecked: Date.now(),
        });
      }
    } catch (e) {
      console.error("Update check failed:", e);
      set({
        status: "error",
        error: e instanceof Error ? e.message : String(e),
        lastChecked: Date.now(),
      });
    }
  },

  downloadAndInstall: async () => {
    const { update } = get();
    if (!update) return;

    set({ status: "downloading", progress: 0 });

    try {
      let totalBytes = 0;
      let downloadedBytes = 0;

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            totalBytes = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloadedBytes += event.data.chunkLength;
            if (totalBytes > 0) {
              set({ progress: Math.round((downloadedBytes / totalBytes) * 100) });
            }
            break;
          case "Finished":
            set({ status: "ready", progress: 100 });
            break;
        }
      });
    } catch (e) {
      console.error("Update download failed:", e);
      set({
        status: "error",
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },

  dismiss: () => {
    set({ dismissed: true });
  },
}));

// Set up periodic update checking
let intervalId: ReturnType<typeof setInterval> | null = null;

export function startAutoUpdateCheck() {
  // Initial check after 10 seconds to avoid blocking startup
  setTimeout(() => {
    useUpdateStore.getState().checkForUpdate();
  }, 10_000);

  // Periodic check every 6 hours
  if (intervalId) clearInterval(intervalId);
  intervalId = setInterval(() => {
    useUpdateStore.getState().checkForUpdate();
  }, CHECK_INTERVAL_MS);
}

export function stopAutoUpdateCheck() {
  if (intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
}
