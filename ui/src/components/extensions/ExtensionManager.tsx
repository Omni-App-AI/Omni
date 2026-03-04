import { useEffect, useState } from "react";
import { useExtensionStore } from "../../stores/extensionStore";
import { ExtensionCard } from "./ExtensionCard";

export function ExtensionManager() {
  const { extensions, loading, loadExtensions, install } = useExtensionStore();
  const [installPath, setInstallPath] = useState("");
  const [showInstall, setShowInstall] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadExtensions();
  }, [loadExtensions]);

  const handleInstall = async () => {
    if (!installPath.trim()) return;
    setInstalling(true);
    setError(null);
    try {
      await install(installPath.trim());
      setInstallPath("");
      setShowInstall(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="flex flex-col gap-6 p-6" style={{ backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Extensions</h1>
        <button
          onClick={() => setShowInstall(!showInstall)}
          className="px-4 py-2 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
          style={{ backgroundColor: "var(--accent)" }}
        >
          Install Extension
        </button>
      </div>

      {showInstall && (
        <div
          className="rounded-lg p-4 flex flex-col gap-3"
          style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border)" }}
        >
          <label className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
            Extension source path
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={installPath}
              onChange={(e) => setInstallPath(e.target.value)}
              placeholder="C:\path\to\extension"
              className="flex-1 px-3 py-2 rounded text-sm outline-none"
              style={{
                backgroundColor: "var(--bg-primary)",
                border: "1px solid var(--border)",
                color: "var(--text-primary)",
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleInstall();
              }}
            />
            <button
              onClick={handleInstall}
              disabled={installing || !installPath.trim()}
              className="px-4 py-2 rounded text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
              style={{ backgroundColor: "var(--success)" }}
            >
              {installing ? "Installing..." : "Install"}
            </button>
          </div>
          {error && (
            <p className="text-sm" style={{ color: "var(--danger)" }}>
              {error}
            </p>
          )}
        </div>
      )}

      {loading ? (
        <div className="text-center py-8" style={{ color: "var(--text-muted)" }}>
          Loading extensions...
        </div>
      ) : extensions.length === 0 ? (
        <div className="text-center py-8" style={{ color: "var(--text-muted)" }}>
          No extensions installed.
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {extensions.map((ext) => (
            <ExtensionCard key={ext.id} extension={ext} />
          ))}
        </div>
      )}
    </div>
  );
}
