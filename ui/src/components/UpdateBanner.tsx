import { useUpdateStore } from "../stores/updateStore";

export function UpdateBanner() {
  const status = useUpdateStore((s) => s.status);
  const version = useUpdateStore((s) => s.version);
  const progress = useUpdateStore((s) => s.progress);
  const dismissed = useUpdateStore((s) => s.dismissed);
  const error = useUpdateStore((s) => s.error);
  const downloadAndInstall = useUpdateStore((s) => s.downloadAndInstall);
  const dismiss = useUpdateStore((s) => s.dismiss);
  const checkForUpdate = useUpdateStore((s) => s.checkForUpdate);

  if (dismissed && (status === "available" || status === "ready" || status === "error")) return null;
  if (status === "idle" || status === "checking") return null;

  return (
    <div className="flex items-center gap-3 px-4 py-2 text-xs border-b border-[var(--border)] bg-[var(--bg-secondary)]">
      {status === "available" && (
        <>
          <span className="text-[var(--text-secondary)]">
            Update available: <strong className="text-[var(--text-primary)]">v{version}</strong>
          </span>
          <button
            onClick={downloadAndInstall}
            className="px-3 py-1 rounded-md text-xs font-medium bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            Download & Install
          </button>
          <button
            onClick={dismiss}
            className="text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors ml-auto"
          >
            Dismiss
          </button>
        </>
      )}

      {status === "downloading" && (
        <>
          <span className="text-[var(--text-secondary)]">
            Downloading update...
          </span>
          <div className="flex-1 max-w-48 h-1.5 rounded-full bg-[var(--border)] overflow-hidden">
            <div
              className="h-full bg-[var(--accent)] transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
          <span className="text-[var(--text-muted)] tabular-nums">{progress}%</span>
        </>
      )}

      {status === "ready" && (
        <>
          <span className="text-[var(--text-secondary)]">
            Update installed. Restart Omni to apply <strong className="text-[var(--text-primary)]">v{version}</strong>.
          </span>
          <button
            onClick={dismiss}
            className="text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors ml-auto"
          >
            Dismiss
          </button>
        </>
      )}

      {status === "error" && (
        <>
          <span className="text-red-400">
            Update check failed: {error}
          </span>
          <button
            onClick={checkForUpdate}
            className="px-3 py-1 rounded-md text-xs font-medium border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
          >
            Retry
          </button>
          <button
            onClick={dismiss}
            className="text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors ml-auto"
          >
            Dismiss
          </button>
        </>
      )}
    </div>
  );
}
