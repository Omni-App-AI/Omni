interface PermissionPromptProps {
  extensionId: string;
  capability: string;
  reason: string;
  onRespond: (decision: string, duration: string) => void;
  onClose: () => void;
}

export function PermissionPrompt({ extensionId, capability, reason, onRespond, onClose }: PermissionPromptProps) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ backgroundColor: "rgba(0, 0, 0, 0.6)" }}
      onClick={onClose}
    >
      <div
        className="w-full max-w-md mx-4 rounded-lg shadow-xl"
        style={{ backgroundColor: "var(--bg-primary)", border: "1px solid var(--border)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className="flex items-center justify-between px-5 py-4"
          style={{ borderBottom: "1px solid var(--border)" }}
        >
          <h2 className="text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
            Permission Request
          </h2>
          <button
            onClick={onClose}
            className="text-xl leading-none px-1 transition-colors"
            style={{ color: "var(--text-muted)" }}
            onMouseEnter={(e) => (e.currentTarget.style.color = "var(--text-primary)")}
            onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-muted)")}
          >
            &times;
          </button>
        </div>

        <div className="px-5 py-4 flex flex-col gap-3" style={{ color: "var(--text-primary)" }}>
          <div>
            <span className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
              Extension
            </span>
            <p className="font-mono text-sm mt-0.5">{extensionId}</p>
          </div>

          <div>
            <span className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
              Capability
            </span>
            <p
              className="mt-0.5 inline-block px-2 py-0.5 rounded text-sm font-medium"
              style={{ backgroundColor: "var(--accent)", color: "white" }}
            >
              {capability}
            </p>
          </div>

          <div>
            <span className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
              Reason
            </span>
            <p
              className="mt-0.5 text-sm p-2 rounded"
              style={{ backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)" }}
            >
              {reason}
            </p>
          </div>
        </div>

        <div
          className="px-5 py-4 flex flex-wrap gap-2"
          style={{ borderTop: "1px solid var(--border)" }}
        >
          <button
            onClick={() => onRespond("allow", "once")}
            className="px-3 py-1.5 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
            style={{ backgroundColor: "var(--success)" }}
          >
            Allow Once
          </button>
          <button
            onClick={() => onRespond("allow", "session")}
            className="px-3 py-1.5 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
            style={{ backgroundColor: "var(--success)" }}
          >
            Allow Session
          </button>
          <button
            onClick={() => onRespond("allow", "always")}
            className="px-3 py-1.5 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
            style={{ backgroundColor: "var(--success)" }}
          >
            Allow Always
          </button>
          <button
            onClick={() => onRespond("deny", "once")}
            className="px-3 py-1.5 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
            style={{ backgroundColor: "var(--danger)" }}
          >
            Deny Once
          </button>
          <button
            onClick={() => onRespond("deny", "always")}
            className="px-3 py-1.5 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
            style={{ backgroundColor: "var(--danger)" }}
          >
            Deny Always
          </button>
        </div>
      </div>
    </div>
  );
}
