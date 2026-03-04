import { create } from "zustand";
import { updateSettings, getSettings } from "../lib/tauri-commands";

export type Theme = "light" | "dark" | "system";
export type FontFamily = "system" | "inter" | "jetbrains-mono" | "fira-code" | "source-sans-3";
export type LineHeight = "compact" | "normal" | "relaxed";
export type UiDensity = "compact" | "comfortable" | "spacious";
export type MessageStyle = "bubbles" | "flat" | "compact";
export type CodeTheme = "dark" | "light" | "auto";

// ─── Color Utility ─────────────────────────────────────────────────

function hexToHsl(hex: string): [number, number, number] {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return [h * 360, s, l];
}

function hslToHex(h: number, s: number, l: number): string {
  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  h /= 360;
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const r = Math.round(hue2rgb(p, q, h + 1 / 3) * 255);
  const g = Math.round(hue2rgb(p, q, h) * 255);
  const b = Math.round(hue2rgb(p, q, h - 1 / 3) * 255);
  return `#${[r, g, b].map((x) => x.toString(16).padStart(2, "0")).join("")}`;
}

function darkenHex(hex: string, percent: number): string {
  const [h, s, l] = hexToHsl(hex);
  return hslToHex(h, s, Math.max(0, l - percent / 100));
}

// ─── Font Family Map ───────────────────────────────────────────────

export const FONT_FAMILY_MAP: Record<FontFamily, string> = {
  system: "system-ui, -apple-system, sans-serif",
  inter: "'Inter', system-ui, sans-serif",
  "jetbrains-mono": "'JetBrains Mono', monospace",
  "fira-code": "'Fira Code', monospace",
  "source-sans-3": "'Source Sans 3', system-ui, sans-serif",
};

const LINE_HEIGHT_MAP: Record<LineHeight, string> = {
  compact: "1.4",
  normal: "1.5",
  relaxed: "1.7",
};

const DENSITY_MAP: Record<UiDensity, { padding: string; gap: string }> = {
  compact: { padding: "0.5rem", gap: "0.375rem" },
  comfortable: { padding: "1rem", gap: "0.75rem" },
  spacious: { padding: "1.5rem", gap: "1rem" },
};

// ─── Default Values ────────────────────────────────────────────────

const DEFAULTS = {
  theme: "system" as Theme,
  fontSize: 14,
  showActionFeed: true,
  accentColor: "#3b82f6",
  fontFamily: "system" as FontFamily,
  lineHeight: "normal" as LineHeight,
  uiDensity: "comfortable" as UiDensity,
  sidebarWidth: 250,
  messageStyle: "bubbles" as MessageStyle,
  maxMessageWidth: 75,
  codeTheme: "dark" as CodeTheme,
  showTimestamps: false,
  borderRadius: 8,
  reduceAnimations: false,
  highContrast: false,
  autoUpdate: true,
};

// ─── Store Interface ───────────────────────────────────────────────

interface SettingsState {
  theme: Theme;
  fontSize: number;
  showActionFeed: boolean;
  accentColor: string;
  fontFamily: FontFamily;
  lineHeight: LineHeight;
  uiDensity: UiDensity;
  sidebarWidth: number;
  messageStyle: MessageStyle;
  maxMessageWidth: number;
  codeTheme: CodeTheme;
  showTimestamps: boolean;
  borderRadius: number;
  reduceAnimations: boolean;
  highContrast: boolean;
  autoUpdate: boolean;

  setTheme: (theme: Theme) => void;
  setFontSize: (size: number) => void;
  toggleActionFeed: () => void;
  setAccentColor: (color: string) => void;
  setFontFamily: (ff: FontFamily) => void;
  setLineHeight: (lh: LineHeight) => void;
  setUiDensity: (density: UiDensity) => void;
  setSidebarWidth: (width: number) => void;
  setMessageStyle: (style: MessageStyle) => void;
  setMaxMessageWidth: (width: number) => void;
  setCodeTheme: (ct: CodeTheme) => void;
  setShowTimestamps: (show: boolean) => void;
  setBorderRadius: (radius: number) => void;
  setReduceAnimations: (reduce: boolean) => void;
  setHighContrast: (hc: boolean) => void;
  setAutoUpdate: (enabled: boolean) => void;
  resetAppearance: () => void;
  loadSettings: () => Promise<void>;

  applyTheme: () => void;
  applyAppearance: () => void;
}

// ─── Store ─────────────────────────────────────────────────────────

export const useSettingsStore = create<SettingsState>((set, get) => ({
  ...DEFAULTS,

  setTheme: (theme: Theme) => {
    set({ theme });
    get().applyTheme();
    updateSettings({ theme }).catch(console.error);
  },

  setFontSize: (fontSize: number) => {
    set({ fontSize });
    get().applyAppearance();
    updateSettings({ fontSize }).catch(console.error);
  },

  toggleActionFeed: () => {
    const next = !get().showActionFeed;
    set({ showActionFeed: next });
    updateSettings({ showActionFeed: next }).catch(console.error);
  },

  setAccentColor: (accentColor: string) => {
    set({ accentColor });
    get().applyAppearance();
    updateSettings({ accentColor }).catch(console.error);
  },

  setFontFamily: (fontFamily: FontFamily) => {
    set({ fontFamily });
    get().applyAppearance();
    updateSettings({ fontFamily }).catch(console.error);
  },

  setLineHeight: (lineHeight: LineHeight) => {
    set({ lineHeight });
    get().applyAppearance();
    updateSettings({ lineHeight }).catch(console.error);
  },

  setUiDensity: (uiDensity: UiDensity) => {
    set({ uiDensity });
    get().applyAppearance();
    updateSettings({ uiDensity }).catch(console.error);
  },

  setSidebarWidth: (sidebarWidth: number) => {
    set({ sidebarWidth });
    get().applyAppearance();
    updateSettings({ sidebarWidth }).catch(console.error);
  },

  setMessageStyle: (messageStyle: MessageStyle) => {
    set({ messageStyle });
    updateSettings({ messageStyle }).catch(console.error);
  },

  setMaxMessageWidth: (maxMessageWidth: number) => {
    set({ maxMessageWidth });
    get().applyAppearance();
    updateSettings({ maxMessageWidth }).catch(console.error);
  },

  setCodeTheme: (codeTheme: CodeTheme) => {
    set({ codeTheme });
    updateSettings({ codeTheme }).catch(console.error);
  },

  setShowTimestamps: (showTimestamps: boolean) => {
    set({ showTimestamps });
    updateSettings({ showTimestamps }).catch(console.error);
  },

  setBorderRadius: (borderRadius: number) => {
    set({ borderRadius });
    get().applyAppearance();
    updateSettings({ borderRadius }).catch(console.error);
  },

  setReduceAnimations: (reduceAnimations: boolean) => {
    set({ reduceAnimations });
    get().applyAppearance();
    updateSettings({ reduceAnimations }).catch(console.error);
  },

  setHighContrast: (highContrast: boolean) => {
    set({ highContrast });
    get().applyAppearance();
    updateSettings({ highContrast }).catch(console.error);
  },

  setAutoUpdate: (autoUpdate: boolean) => {
    set({ autoUpdate });
    updateSettings({ autoUpdate }).catch(console.error);
  },

  resetAppearance: () => {
    set({ ...DEFAULTS });
    get().applyTheme();
    get().applyAppearance();
    updateSettings({
      theme: DEFAULTS.theme,
      fontSize: DEFAULTS.fontSize,
      accentColor: DEFAULTS.accentColor,
      fontFamily: DEFAULTS.fontFamily,
      lineHeight: DEFAULTS.lineHeight,
      uiDensity: DEFAULTS.uiDensity,
      sidebarWidth: DEFAULTS.sidebarWidth,
      messageStyle: DEFAULTS.messageStyle,
      maxMessageWidth: DEFAULTS.maxMessageWidth,
      codeTheme: DEFAULTS.codeTheme,
      showTimestamps: DEFAULTS.showTimestamps,
      borderRadius: DEFAULTS.borderRadius,
      reduceAnimations: DEFAULTS.reduceAnimations,
      highContrast: DEFAULTS.highContrast,
    }).catch(console.error);
  },

  loadSettings: async () => {
    try {
      const json = await getSettings();
      const ui = JSON.parse(json);
      set({
        theme: ui.theme ?? DEFAULTS.theme,
        fontSize: ui.font_size ?? DEFAULTS.fontSize,
        showActionFeed: ui.show_action_feed ?? DEFAULTS.showActionFeed,
        accentColor: ui.accent_color ?? DEFAULTS.accentColor,
        fontFamily: ui.font_family ?? DEFAULTS.fontFamily,
        lineHeight: ui.line_height ?? DEFAULTS.lineHeight,
        uiDensity: ui.ui_density ?? DEFAULTS.uiDensity,
        sidebarWidth: ui.sidebar_width ?? DEFAULTS.sidebarWidth,
        messageStyle: ui.message_style ?? DEFAULTS.messageStyle,
        maxMessageWidth: ui.max_message_width ?? DEFAULTS.maxMessageWidth,
        codeTheme: ui.code_theme ?? DEFAULTS.codeTheme,
        showTimestamps: ui.show_timestamps ?? DEFAULTS.showTimestamps,
        borderRadius: ui.border_radius ?? DEFAULTS.borderRadius,
        reduceAnimations: ui.reduce_animations ?? DEFAULTS.reduceAnimations,
        highContrast: ui.high_contrast ?? DEFAULTS.highContrast,
        autoUpdate: ui.auto_update ?? DEFAULTS.autoUpdate,
      });
      get().applyTheme();
      get().applyAppearance();
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  },

  applyTheme: () => {
    const { theme } = get();
    if (theme === "system") {
      document.documentElement.removeAttribute("data-theme");
    } else {
      document.documentElement.setAttribute("data-theme", theme);
    }
  },

  applyAppearance: () => {
    const s = get();
    const root = document.documentElement;
    const style = root.style;

    // Font size
    style.setProperty("--font-size", `${s.fontSize}px`);
    root.style.fontSize = `${s.fontSize}px`;

    // Font family
    style.setProperty("--font-family", FONT_FAMILY_MAP[s.fontFamily]);
    root.style.fontFamily = FONT_FAMILY_MAP[s.fontFamily];

    // Line height
    style.setProperty("--line-height", LINE_HEIGHT_MAP[s.lineHeight]);
    root.style.lineHeight = LINE_HEIGHT_MAP[s.lineHeight];

    // Accent color
    style.setProperty("--accent", s.accentColor);
    style.setProperty("--accent-hover", darkenHex(s.accentColor, 15));

    // Border radius
    style.setProperty("--border-radius", `${s.borderRadius}px`);

    // Density
    const d = DENSITY_MAP[s.uiDensity];
    style.setProperty("--density-padding", d.padding);
    style.setProperty("--density-gap", d.gap);

    // Sidebar width
    style.setProperty("--sidebar-width", `${s.sidebarWidth}px`);

    // Max message width
    style.setProperty("--max-message-width", `${s.maxMessageWidth}%`);

    // Reduce animations
    if (s.reduceAnimations) {
      root.classList.add("reduce-motion");
    } else {
      root.classList.remove("reduce-motion");
    }

    // High contrast
    if (s.highContrast) {
      root.classList.add("high-contrast");
    } else {
      root.classList.remove("high-contrast");
    }
  },
}));
