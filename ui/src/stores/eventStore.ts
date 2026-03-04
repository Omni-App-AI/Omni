import { create } from "zustand";

export interface OmniEventEntry {
  id: number;
  timestamp: string;
  eventType: string;
  payload: Record<string, unknown>;
}

let nextId = 0;

interface EventState {
  events: OmniEventEntry[];
  addEvent: (eventType: string, payload: Record<string, unknown>) => void;
  clearEvents: () => void;
}

export const useEventStore = create<EventState>((set) => ({
  events: [],

  addEvent: (eventType: string, payload: Record<string, unknown>) => {
    const entry: OmniEventEntry = {
      id: nextId++,
      timestamp: new Date().toISOString(),
      eventType,
      payload,
    };
    set((s) => ({
      events: [entry, ...s.events].slice(0, 500), // Keep last 500 events
    }));
  },

  clearEvents: () => set({ events: [] }),
}));
