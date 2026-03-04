import { Radio, ArrowRight, MessageCircle, Send, Hash, Phone } from "lucide-react";
import { CHANNEL_META } from "./channelMeta";

interface EmptyChannelStateProps {
  onAddChannel: (preselect?: string) => void;
}

const QUICK_START = [
  { type: "discord", icon: MessageCircle, label: "Discord", hint: "Bot token" },
  { type: "telegram", icon: Send, label: "Telegram", hint: "Bot token" },
  { type: "slack", icon: Hash, label: "Slack", hint: "Bot token" },
  { type: "whatsapp-web", icon: Phone, label: "WhatsApp", hint: "QR code scan" },
] as const;

export function EmptyChannelState({ onAddChannel }: EmptyChannelStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 px-6 gap-8">
      {/* Hero */}
      <div className="flex flex-col items-center gap-4 max-w-md text-center">
        <div
          className="w-16 h-16 rounded-2xl flex items-center justify-center"
          style={{
            backgroundColor: "color-mix(in srgb, var(--accent) 15%, transparent)",
          }}
        >
          <Radio size={32} style={{ color: "var(--accent)" }} />
        </div>

        <h2 className="text-xl font-bold" style={{ color: "var(--text-primary)" }}>
          Connect your first messaging platform
        </h2>
        <p className="text-sm leading-relaxed" style={{ color: "var(--text-muted)" }}>
          Omni lets your AI extensions communicate through Discord, Telegram, Slack,
          and {Object.keys(CHANNEL_META).length - 3} other platforms.
        </p>

        <button
          onClick={() => onAddChannel()}
          className="flex items-center gap-2 px-6 py-3 rounded-lg text-sm font-semibold transition-opacity hover:opacity-90"
          style={{ backgroundColor: "var(--accent)", color: "white" }}
        >
          Add Channel
          <ArrowRight size={16} />
        </button>
      </div>

      {/* Quick-start divider */}
      <div className="flex items-center gap-3 w-full max-w-lg">
        <div className="flex-1 h-px" style={{ backgroundColor: "var(--border)" }} />
        <span className="text-xs font-medium" style={{ color: "var(--text-muted)" }}>
          or start with a popular channel
        </span>
        <div className="flex-1 h-px" style={{ backgroundColor: "var(--border)" }} />
      </div>

      {/* Quick-start cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 w-full max-w-lg">
        {QUICK_START.map(({ type, icon: Icon, label, hint }) => (
          <button
            key={type}
            onClick={() => onAddChannel(type)}
            className="flex flex-col items-center gap-2 p-4 rounded-lg transition-colors text-center"
            style={{
              backgroundColor: "var(--bg-secondary)",
              border: "1px solid var(--border)",
            }}
            onMouseEnter={(e) =>
              (e.currentTarget.style.borderColor = "var(--accent)")
            }
            onMouseLeave={(e) =>
              (e.currentTarget.style.borderColor = "var(--border)")
            }
          >
            <Icon size={24} style={{ color: "var(--accent)" }} />
            <span
              className="text-sm font-medium"
              style={{ color: "var(--text-primary)" }}
            >
              {label}
            </span>
            <span className="text-xs" style={{ color: "var(--text-muted)" }}>
              {hint}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
