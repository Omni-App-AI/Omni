import { useMemo, useState } from "react";
import { html as diff2html } from "diff2html";
import "diff2html/bundles/css/diff2html.min.css";
import DOMPurify from "dompurify";

interface DiffViewerProps {
  diff: string;
}

export function DiffViewer({ diff }: DiffViewerProps) {
  const [sideBySide, setSideBySide] = useState(false);

  const diffHtml = useMemo(() => {
    if (!diff || !diff.trim()) return "";
    const raw = diff2html(diff, {
      drawFileList: false,
      matching: "lines",
      outputFormat: sideBySide ? "side-by-side" : "line-by-line",
    });
    return DOMPurify.sanitize(raw);
  }, [diff, sideBySide]);

  if (!diffHtml) {
    return <div className="text-xs text-[var(--text-muted)] p-2">No diff content</div>;
  }

  return (
    <div className="diff-viewer">
      <div className="flex items-center justify-end mb-1">
        <button
          onClick={() => setSideBySide(!sideBySide)}
          aria-label={sideBySide ? "Switch to unified view" : "Switch to side-by-side view"}
          aria-pressed={sideBySide}
          className="text-xs px-2 py-0.5 rounded bg-[var(--bg-hover)] text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
        >
          {sideBySide ? "Unified" : "Side-by-side"}
        </button>
      </div>
      <div
        className="diff-content rounded overflow-auto max-h-[60vh] text-xs"
        role="region"
        aria-label="Diff content"
        dangerouslySetInnerHTML={{ __html: diffHtml }}
      />
    </div>
  );
}
