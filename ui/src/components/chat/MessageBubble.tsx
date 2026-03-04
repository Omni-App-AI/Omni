import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, Brain } from "lucide-react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import { oneLight } from "react-syntax-highlighter/dist/esm/styles/prism";
import { useSettingsStore } from "../../stores/settingsStore";

interface MessageBubbleProps {
  role: string;
  content: string;
  isStreaming?: boolean;
  timestamp?: string;
  /** Thinking text from extended/adaptive thinking (assistant messages only). */
  thinkingContent?: string;
}

/** Strip <think>...</think> blocks (and unclosed trailing <think>) from model output. */
function stripThinkTags(text: string): { cleaned: string; thinking: string } {
  let thinking = "";
  // Extract completed <think>...</think> blocks
  const thinkRegex = /<think>([\s\S]*?)<\/think>/g;
  let match;
  while ((match = thinkRegex.exec(text)) !== null) {
    thinking += (thinking ? "\n" : "") + match[1].trim();
  }
  // Remove completed blocks
  let cleaned = text.replace(/<think>[\s\S]*?<\/think>/g, "");
  // Remove unclosed <think> at end (ongoing thinking during streaming)
  const unclosed = cleaned.match(/<think>([\s\S]*)$/);
  if (unclosed) {
    thinking += (thinking ? "\n" : "") + unclosed[1].trim();
    cleaned = cleaned.replace(/<think>[\s\S]*$/, "");
  }
  return { cleaned: cleaned.trim(), thinking };
}

export function MessageBubble({ role, content, isStreaming, timestamp, thinkingContent }: MessageBubbleProps) {
  const isUser = role === "user";
  const [thinkingExpanded, setThinkingExpanded] = useState(false);
  const messageStyle = useSettingsStore((s) => s.messageStyle);
  const codeTheme = useSettingsStore((s) => s.codeTheme);
  const showTimestamps = useSettingsStore((s) => s.showTimestamps);
  const theme = useSettingsStore((s) => s.theme);

  // Strip <think> tags from content and extract thinking text
  const { cleaned: displayContent, thinking: extractedThinking } = useMemo(
    () => (isUser ? { cleaned: content, thinking: "" } : stripThinkTags(content)),
    [content, isUser],
  );
  const effectiveThinking = thinkingContent || extractedThinking;

  const thinkingWordCount = useMemo(() => {
    if (!effectiveThinking) return 0;
    return effectiveThinking.trim().split(/\s+/).filter(Boolean).length;
  }, [effectiveThinking]);

  // Determine effective code highlight theme
  const codeStyle = useMemo(() => {
    if (codeTheme === "dark") return oneDark;
    if (codeTheme === "light") return oneLight;
    // "auto" - check effective theme
    if (theme === "dark") return oneDark;
    if (theme === "light") return oneLight;
    // System theme - check media query
    if (typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches) {
      return oneDark;
    }
    return oneLight;
  }, [codeTheme, theme]);

  // Style variants
  const getContainerClasses = () => {
    switch (messageStyle) {
      case "flat":
        return "mb-0 px-4";
      case "compact":
        return `flex ${isUser ? "justify-end" : "justify-start"} mb-1 px-4`;
      default: // bubbles
        return `flex ${isUser ? "justify-end" : "justify-start"} mb-3 px-4`;
    }
  };

  const getBubbleClasses = () => {
    switch (messageStyle) {
      case "flat":
        return `relative w-full px-4 py-3 text-sm leading-relaxed border-b border-[var(--border)] ${
          isUser
            ? "bg-[color-mix(in_srgb,var(--accent)_8%,transparent)] text-[var(--text-primary)]"
            : "bg-transparent text-[var(--text-primary)]"
        }`;
      case "compact":
        return `relative px-3 py-1.5 text-sm leading-snug ${
          isUser
            ? "bg-[var(--accent)] text-white rounded-lg rounded-br-sm"
            : "bg-[var(--bg-secondary)] text-[var(--text-primary)] border border-[var(--border)] rounded-lg rounded-bl-sm"
        }`;
      default: // bubbles
        return `relative rounded-xl px-4 py-3 text-sm leading-relaxed ${
          isUser
            ? "bg-[var(--accent)] text-white rounded-br-sm"
            : "bg-[var(--bg-secondary)] text-[var(--text-primary)] border border-[var(--border)] rounded-bl-sm"
        }`;
    }
  };

  const getBubbleStyle = (): React.CSSProperties => {
    if (messageStyle === "flat") return {};
    return { maxWidth: "var(--max-message-width)" };
  };

  const formattedTime = useMemo(() => {
    if (!timestamp) return null;
    try {
      return new Date(timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } catch {
      return null;
    }
  }, [timestamp]);

  return (
    <div className={getContainerClasses()}>
      <div className={getBubbleClasses()} style={getBubbleStyle()}>
        {effectiveThinking && !isUser && (
          <div className="mb-2">
            <button
              onClick={() => setThinkingExpanded((prev) => !prev)}
              className="flex items-center gap-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
            >
              <Brain size={12} />
              {thinkingExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              <span>Thinking ({thinkingWordCount} words)</span>
            </button>
            {thinkingExpanded && (
              <pre className="mt-1.5 text-xs font-mono bg-[var(--bg-primary)] border border-[var(--border)] rounded p-2 overflow-x-auto max-h-64 overflow-y-auto text-[var(--text-muted)] whitespace-pre-wrap">
                {effectiveThinking}
              </pre>
            )}
          </div>
        )}

        <div className="prose prose-sm max-w-none break-words [&_p]:m-0 [&_p+p]:mt-2 [&_pre]:my-2 [&_ul]:my-1 [&_ol]:my-1">
          <Markdown
            remarkPlugins={[remarkGfm]}
            components={{
              code({ className, children, ...rest }) {
                const match = /language-(\w+)/.exec(className || "");
                const codeString = String(children).replace(/\n$/, "");

                if (match) {
                  return (
                    <SyntaxHighlighter
                      style={codeStyle}
                      language={match[1]}
                      PreTag="div"
                      customStyle={{
                        margin: 0,
                        borderRadius: "0.375rem",
                        fontSize: "0.8125rem",
                      }}
                    >
                      {codeString}
                    </SyntaxHighlighter>
                  );
                }

                return (
                  <code
                    className={`${className ?? ""} rounded bg-[var(--bg-primary)] px-1.5 py-0.5 text-xs font-mono`}
                    {...rest}
                  >
                    {children}
                  </code>
                );
              },
            }}
          >
            {displayContent}
          </Markdown>
        </div>

        {isStreaming && (
          <span className="inline-block ml-1 h-3 w-0.5 bg-[var(--text-primary)] animate-pulse" />
        )}

        {showTimestamps && formattedTime && (
          <span className={`block text-[10px] mt-1 ${
            isUser && messageStyle !== "flat"
              ? "text-white/60"
              : "text-[var(--text-muted)]"
          }`}>
            {formattedTime}
          </span>
        )}
      </div>
    </div>
  );
}
