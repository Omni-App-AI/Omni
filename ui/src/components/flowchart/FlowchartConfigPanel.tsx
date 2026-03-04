import { useMemo, useCallback, useState } from "react";
import { useFlowchartStore } from "../../stores/flowchartStore";

interface FlowchartConfigPanelProps {
  selectedNodeId: string | null;
}

// Expression path hints (E2)
const EXPRESSION_HINTS = [
  { path: "$.params.<key>", desc: "Tool parameter value" },
  { path: "$.nodes.<id>.<field>", desc: "Output from a previous node" },
  { path: "$var.<name>", desc: "Variable set by SetVariable" },
  { path: "$var.loop_item", desc: "Current loop item" },
  { path: "$var.loop_index", desc: "Current loop index" },
  { path: "{{...}}", desc: "Template interpolation" },
];

export function FlowchartConfigPanel({
  selectedNodeId,
}: FlowchartConfigPanelProps) {
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const updateActive = useFlowchartStore((s) => s.updateActive);

  const node = useMemo(() => {
    if (!selectedNodeId || !activeFlowchart) return null;
    return (activeFlowchart.nodes as Array<Record<string, unknown>>).find(
      (n) => n.id === selectedNodeId,
    );
  }, [selectedNodeId, activeFlowchart]);

  const updateNode = useCallback(
    (updates: Record<string, unknown>) => {
      if (!activeFlowchart || !selectedNodeId) return;
      const nodes = (
        activeFlowchart.nodes as Array<Record<string, unknown>>
      ).map((n) => (n.id === selectedNodeId ? { ...n, ...updates } : n));
      updateActive({ nodes });
    },
    [activeFlowchart, selectedNodeId, updateActive],
  );

  const updateConfig = useCallback(
    (key: string, value: unknown) => {
      if (!node) return;
      const config = { ...(node.config as Record<string, unknown>), [key]: value };
      updateNode({ config });
    },
    [node, updateNode],
  );

  if (!node) {
    return (
      <div className="p-3 text-xs text-[var(--text-muted)]">
        Select a node to configure it.
      </div>
    );
  }

  const nodeType = node.node_type as string;
  const config = (node.config as Record<string, unknown>) ?? {};

  // Determine which nodes support timeout/retry (action nodes)
  const supportsTimeoutRetry = [
    "http_request", "llm_request", "channel_send", "native_tool", "sub_flow",
  ].includes(nodeType);

  return (
    <div className="p-3 space-y-3 overflow-y-auto">
      <div>
        <label className="text-xs text-[var(--text-muted)] block mb-1">
          Label
        </label>
        <input
          className="w-full px-2 py-1 text-sm rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
          value={(node.label as string) ?? ""}
          onChange={(e) => updateNode({ label: e.target.value })}
        />
      </div>

      <div className="text-xs text-[var(--text-muted)] uppercase tracking-wider">
        {nodeType.replace(/_/g, " ")} Config
      </div>

      {nodeType === "http_request" && (
        <>
          <ConfigInput label="URL" value={config.url} onChange={(v) => updateConfig("url", v)} placeholder="https://api.example.com/..." validate={validateUrl} />
          <ConfigSelect label="Method" value={config.method} onChange={(v) => updateConfig("method", v)} options={["GET", "POST", "PUT", "PATCH", "DELETE"]} />
          <ConfigTextarea label="Headers (JSON)" value={config.headers ? JSON.stringify(config.headers, null, 2) : ""} onChange={(v) => { try { updateConfig("headers", JSON.parse(v)); } catch {} }} placeholder='{"Authorization": "Bearer ..."}' validate={validateJson} />
          <ConfigTextarea label="Body Template" value={config.body_template} onChange={(v) => updateConfig("body_template", v)} placeholder='{"key": "{{$.params.value}}"}' />
        </>
      )}

      {nodeType === "llm_request" && (
        <>
          <ConfigTextarea label="Prompt Template" value={config.prompt_template} onChange={(v) => updateConfig("prompt_template", v)} placeholder="Summarize: {{$.params.text}}" rows={4} />
          <ConfigInput label="Max Tokens" value={config.max_tokens} onChange={(v) => { const n = parseInt(v); updateConfig("max_tokens", Number.isNaN(n) ? 1024 : Math.max(1, n)); }} type="number" />
        </>
      )}

      {nodeType === "condition" && (
        <>
          <ConfigInput label="Expression" value={config.expression} onChange={(v) => updateConfig("expression", v)} placeholder="$.params.count > 10" />
          <ExpressionHints />
        </>
      )}

      {nodeType === "switch" && (
        <>
          <ConfigInput label="Expression" value={config.expression} onChange={(v) => updateConfig("expression", v)} placeholder="$.params.status" />
          <SwitchCasesEditor
            cases={Array.isArray(config.cases) ? config.cases as Array<{ value: string; label?: string }> : []}
            onChange={(cases) => updateConfig("cases", cases)}
          />
          <ExpressionHints />
        </>
      )}

      {nodeType === "sub_flow" && (
        <>
          <ConfigInput label="Flowchart ID" value={config.flowchart_id} onChange={(v) => updateConfig("flowchart_id", v)} placeholder="flow.user.my_subflow" />
          <ConfigInput label="Tool Name" value={config.tool_name} onChange={(v) => updateConfig("tool_name", v)} placeholder="main" />
          <ConfigTextarea label="Parameters (JSON)" value={config.params_json ? (typeof config.params_json === "string" ? config.params_json : JSON.stringify(config.params_json, null, 2)) : ""} onChange={(v) => { try { updateConfig("params_json", JSON.parse(v)); } catch { updateConfig("params_json", v); } }} placeholder={'{"input": "{{$.params.data}}"}'} rows={4} validate={validateJson} />
        </>
      )}

      {nodeType === "comment" && (
        <ConfigTextarea label="Comment Text" value={config.text} onChange={(v) => updateConfig("text", v)} placeholder="Describe what this section of the flow does..." rows={4} />
      )}

      {nodeType === "transform" && (
        <>
          <ConfigSelect label="Type" value={config.transform_type} onChange={(v) => updateConfig("transform_type", v)} options={["json_path", "template", "regex", "json_build"]} />
          <ConfigInput label="Expression" value={config.expression} onChange={(v) => updateConfig("expression", v)} placeholder="$.data.items" />
          {((config.transform_type as string) === "template" || (config.transform_type as string) === "json_build") && (
            <ConfigTextarea label="Template" value={config.template ? (typeof config.template === "string" ? config.template : JSON.stringify(config.template, null, 2)) : ""} onChange={(v) => { if ((config.transform_type as string) === "json_build") { try { updateConfig("template", JSON.parse(v)); } catch {} } else { updateConfig("template", v); } }} placeholder={(config.transform_type as string) === "json_build" ? '{"name": "$.params.name", "greeting": "Hello {{$.params.name}}!"}' : "Hello {{$.params.name}}!"} rows={4} validate={(config.transform_type as string) === "json_build" ? validateJson : undefined} />
          )}
          {(config.transform_type as string) === "regex" && (
            <ConfigInput label="Input Path" value={config.input_path} onChange={(v) => updateConfig("input_path", v)} placeholder="$.params.text" />
          )}
          <ExpressionHints />
        </>
      )}

      {nodeType === "channel_send" && (
        <>
          <ConfigInput label="Channel ID" value={config.channel_id} onChange={(v) => updateConfig("channel_id", v)} placeholder="discord:main" />
          <ConfigInput label="Recipient" value={config.recipient_template} onChange={(v) => updateConfig("recipient_template", v)} placeholder="{{$.params.recipient}}" />
          <ConfigTextarea label="Message Template" value={config.message_template} onChange={(v) => updateConfig("message_template", v)} placeholder="Result: {{$.nodes.transform_1.output}}" />
        </>
      )}

      {nodeType === "storage_op" && (
        <>
          <ConfigSelect label="Operation" value={config.operation} onChange={(v) => updateConfig("operation", v)} options={["get", "set", "delete"]} />
          <ConfigInput label="Key Template" value={config.key_template} onChange={(v) => updateConfig("key_template", v)} placeholder="my_data_key" />
          {(config.operation as string) === "set" && (
            <ConfigInput label="Value Template" value={config.value_template} onChange={(v) => updateConfig("value_template", v)} placeholder="$.params.data" />
          )}
        </>
      )}

      {nodeType === "config_get" && (
        <ConfigInput label="Config Key" value={config.key} onChange={(v) => updateConfig("key", v)} placeholder="api_endpoint" />
      )}

      {nodeType === "set_variable" && (
        <>
          <ConfigInput label="Variable Name" value={config.variable_name} onChange={(v) => updateConfig("variable_name", v)} placeholder="myVar" validate={validateVarName} />
          <ConfigInput label="Value Expression" value={config.value_expression} onChange={(v) => updateConfig("value_expression", v)} placeholder="$.params.input" />
          <ExpressionHints />
        </>
      )}

      {nodeType === "loop" && (
        <>
          <ConfigInput label="Array Path" value={config.array_path} onChange={(v) => updateConfig("array_path", v)} placeholder="$.params.items" />
          <ConfigInput label="Max Iterations" value={config.max_iterations} onChange={(v) => { const n = parseInt(v); updateConfig("max_iterations", Number.isNaN(n) ? 100 : Math.max(1, n)); }} type="number" />
          <p className="text-[10px] text-[var(--text-muted)]">
            Access current item via <code className="bg-[var(--bg-hover)] px-0.5 rounded">$var.loop_item</code> and index via <code className="bg-[var(--bg-hover)] px-0.5 rounded">$var.loop_index</code>
          </p>
        </>
      )}

      {nodeType === "delay" && (
        <ConfigInput label="Duration (ms)" value={config.milliseconds} onChange={(v) => { const n = parseInt(v); updateConfig("milliseconds", Number.isNaN(n) ? 1000 : Math.max(0, Math.min(30000, n))); }} type="number" />
      )}

      {nodeType === "log" && (
        <>
          <ConfigSelect label="Level" value={config.level} onChange={(v) => updateConfig("level", v)} options={["info", "debug", "warn", "error"]} />
          <ConfigInput label="Message Template" value={config.message_template} onChange={(v) => updateConfig("message_template", v)} placeholder="Processing: {{$.params.id}}" />
        </>
      )}

      {nodeType === "output" && (
        <>
          <ConfigTextarea label="Result Template" value={config.result_template} onChange={(v) => updateConfig("result_template", v)} placeholder="{{$.nodes.transform_1.result}}" />
          <ExpressionHints />
        </>
      )}

      {nodeType === "error_handler" && (
        <ConfigTextarea label="Fallback Value (JSON)" value={config.fallback_value ? JSON.stringify(config.fallback_value, null, 2) : ""} onChange={(v) => { try { updateConfig("fallback_value", JSON.parse(v)); } catch {} }} placeholder='{"error": "An error occurred"}' validate={validateJson} />
      )}

      {nodeType === "merge" && (
        <>
          <ConfigSelect label="Strategy" value={config.strategy} onChange={(v) => updateConfig("strategy", v)} options={["merge_objects", "array_concat", "first_non_null"]} />
          <ConfigTextarea label="Input Paths (one per line)" value={Array.isArray(config.input_paths) ? (config.input_paths as string[]).join("\n") : ""} onChange={(v) => updateConfig("input_paths", v.split("\n").map(s => s.trim()).filter(Boolean))} placeholder={"$.nodes.branch_a\n$.nodes.branch_b"} rows={3} />
        </>
      )}

      {nodeType === "native_tool" && (
        <>
          <ConfigSelect label="Tool" value={config.tool_name} onChange={(v) => updateConfig("tool_name", v)} options={[
            "exec", "read_file", "write_file", "edit_file", "list_files", "apply_patch", "grep_search",
            "web_fetch", "web_search", "web_scrape",
            "memory_save", "memory_search", "memory_get",
            "image_analyze",
            "send_message", "list_channels",
            "notify", "cron_schedule",
            "app_interact",
            "git", "test_runner",
            "clipboard", "code_search",
            "agent_spawn",
            "session_list", "session_history",
          ]} />
          <ConfigTextarea label="Parameters (JSON)" value={config.params_json ? JSON.stringify(config.params_json, null, 2) : ""} onChange={(v) => { try { updateConfig("params_json", JSON.parse(v)); } catch {} }} placeholder={'{\n  "path": "{{$.params.file_path}}"\n}'} rows={6} validate={validateJson} />
          <p className="text-[10px] text-[var(--text-muted)]">
            Use <code className="bg-[var(--bg-hover)] px-0.5 rounded">{"{{$.params.x}}"}</code> in JSON string values for dynamic parameters. All string values are template-expanded.
          </p>
        </>
      )}

      {nodeType === "trigger" && (
        <p className="text-xs text-[var(--text-muted)]">
          Trigger nodes receive tool parameters. Configure the tool schema in the
          Tools tab.
        </p>
      )}

      {/* Per-node timeout + retry settings (D3/D4) */}
      {supportsTimeoutRetry && (
        <div className="border-t border-[var(--border)] pt-3 mt-3 space-y-2">
          <div className="text-xs text-[var(--text-muted)] uppercase tracking-wider">
            Advanced
          </div>
          <ConfigInput
            label="Timeout (ms, 0 = global default)"
            value={config.timeout_ms}
            onChange={(v) => { const n = parseInt(v); updateConfig("timeout_ms", Number.isNaN(n) ? 0 : Math.max(0, n)); }}
            type="number"
          />
          <ConfigInput
            label="Retry Count (0 = no retry)"
            value={config.retry_count}
            onChange={(v) => { const n = parseInt(v); updateConfig("retry_count", Number.isNaN(n) ? 0 : Math.max(0, Math.min(10, n))); }}
            type="number"
          />
          {Number(config.retry_count) > 0 && (
            <ConfigInput
              label="Retry Delay (ms)"
              value={config.retry_delay_ms}
              onChange={(v) => { const n = parseInt(v); updateConfig("retry_delay_ms", Number.isNaN(n) ? 1000 : Math.max(0, n)); }}
              type="number"
            />
          )}
        </div>
      )}
    </div>
  );
}

// ── Validation helpers (E1) ───────────────────────────────────────

function validateUrl(value: string): string | null {
  if (!value) return null;
  // Allow template expressions
  if (value.includes("{{")) return null;
  try {
    new URL(value);
    return null;
  } catch {
    return "Invalid URL format";
  }
}

function validateJson(value: string): string | null {
  if (!value) return null;
  // Allow template expressions in JSON values
  if (value.includes("{{")) return null;
  try {
    JSON.parse(value);
    return null;
  } catch {
    return "Invalid JSON";
  }
}

function validateVarName(value: string): string | null {
  if (!value) return null;
  if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(value)) return null;
  return "Must start with a letter/underscore, alphanumeric only";
}

// ── Expression hints panel (E2) ───────────────────────────────────

function ExpressionHints() {
  const [open, setOpen] = useState(false);
  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="text-[10px] text-[var(--accent)] hover:underline"
      >
        {open ? "Hide" : "Show"} expression reference
      </button>
      {open && (
        <div className="mt-1 p-2 rounded border border-[var(--border)] bg-[var(--bg-primary)] space-y-0.5">
          {EXPRESSION_HINTS.map((h) => (
            <div key={h.path} className="flex gap-2 text-[10px]">
              <code className="text-[var(--accent)] whitespace-nowrap">{h.path}</code>
              <span className="text-[var(--text-muted)]">{h.desc}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Switch cases editor (C6) ──────────────────────────────────────

function SwitchCasesEditor({
  cases,
  onChange,
}: {
  cases: Array<{ value: string; label?: string }>;
  onChange: (cases: Array<{ value: string; label?: string }>) => void;
}) {
  return (
    <div>
      <label className="text-xs text-[var(--text-muted)] block mb-1">
        Cases
      </label>
      <div className="space-y-1">
        {cases.map((c, i) => (
          <div key={i} className="flex gap-1">
            <input
              className="flex-1 px-2 py-1 text-xs rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
              value={c.value}
              placeholder="Value"
              onChange={(e) => {
                const updated = [...cases];
                updated[i] = { ...c, value: e.target.value };
                onChange(updated);
              }}
            />
            <input
              className="w-20 px-2 py-1 text-xs rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
              value={c.label ?? ""}
              placeholder="Label"
              onChange={(e) => {
                const updated = [...cases];
                updated[i] = { ...c, label: e.target.value || undefined };
                onChange(updated);
              }}
            />
            <button
              type="button"
              onClick={() => onChange(cases.filter((_, j) => j !== i))}
              className="px-1.5 text-xs text-red-400 hover:text-red-300"
            >
              x
            </button>
          </div>
        ))}
      </div>
      <button
        type="button"
        onClick={() => onChange([...cases, { value: "" }])}
        className="mt-1 text-[10px] text-[var(--accent)] hover:underline"
      >
        + Add case
      </button>
      <p className="text-[10px] text-[var(--text-muted)] mt-1">
        Non-matching values go to the <strong>Default</strong> branch.
      </p>
    </div>
  );
}

// ── Reusable form components ──────────────────────────────────────

function ConfigInput({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  validate,
}: {
  label: string;
  value: unknown;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  validate?: (v: string) => string | null;
}) {
  const strVal = String(value ?? "");
  const error = validate ? validate(strVal) : null;

  return (
    <div>
      <label className="text-xs text-[var(--text-muted)] block mb-1">
        {label}
      </label>
      <input
        className={`w-full px-2 py-1 text-sm rounded border bg-[var(--bg-primary)] text-[var(--text-primary)] outline-none focus:border-[var(--accent)] ${
          error ? "border-red-400" : "border-[var(--border)]"
        }`}
        value={strVal}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        type={type}
      />
      {error && (
        <p className="text-[10px] text-red-400 mt-0.5">{error}</p>
      )}
    </div>
  );
}

function ConfigTextarea({
  label,
  value,
  onChange,
  placeholder,
  rows = 3,
  validate,
}: {
  label: string;
  value: unknown;
  onChange: (v: string) => void;
  placeholder?: string;
  rows?: number;
  validate?: (v: string) => string | null;
}) {
  const strVal = String(value ?? "");
  const error = validate ? validate(strVal) : null;

  return (
    <div>
      <label className="text-xs text-[var(--text-muted)] block mb-1">
        {label}
      </label>
      <textarea
        className={`w-full px-2 py-1 text-sm rounded border bg-[var(--bg-primary)] text-[var(--text-primary)] outline-none focus:border-[var(--accent)] resize-y ${
          error ? "border-red-400" : "border-[var(--border)]"
        }`}
        value={strVal}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        rows={rows}
      />
      {error && (
        <p className="text-[10px] text-red-400 mt-0.5">{error}</p>
      )}
    </div>
  );
}

function ConfigSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: unknown;
  onChange: (v: string) => void;
  options: string[];
}) {
  return (
    <div>
      <label className="text-xs text-[var(--text-muted)] block mb-1">
        {label}
      </label>
      <select
        className="w-full px-2 py-1 text-sm rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
        value={String(value ?? options[0])}
        onChange={(e) => onChange(e.target.value)}
      >
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </select>
    </div>
  );
}
