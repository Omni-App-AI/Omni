import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Visual Flowchart Builder — No-Code AI Automation",
  description:
    "Build AI agent workflows visually with 19 node types, a drag-and-drop React Flow editor, expression evaluator, auto-triggers, sub-flows, and full access to all 29 native tools — no code required.",
  openGraph: {
    title: "Omni Flowchart Builder — Visual No-Code AI Agent Automation",
    description:
      "Build AI agent workflows visually with 19 node types, drag-and-drop editor, expression evaluator, auto-triggers, sub-flows, and access to all 29 native tools.",
    url: "/docs/flowcharts",
  },
  alternates: { canonical: "/docs/flowcharts" },
};

const nodeCategories = [
  {
    category: "Control Flow",
    nodes: [
      {
        name: "Trigger",
        color: "#22c55e",
        desc: "Entry point that receives tool parameters. Every flowchart tool starts with a Trigger node. Parameters are accessible downstream via $.params.",
      },
      {
        name: "Condition",
        color: "#f59e0b",
        desc: "Evaluates a boolean expression and branches to \"true\" or \"false\" handles. Supports operators like ==, !=, >, <, contains, starts_with, matches, and boolean logic (&&, ||, !).",
      },
      {
        name: "Switch",
        color: "#d946ef",
        desc: "Multi-way branching. Evaluates an expression against a list of cases and routes to the matching case handle (case_0, case_1, ...) or a default handle.",
      },
      {
        name: "Loop",
        color: "#8b5cf6",
        desc: "Iterates over an array. Sets $var.loop_item and $var.loop_index for use in the loop body. Configurable max_iterations (default 100) to prevent infinite loops.",
      },
      {
        name: "Merge",
        color: "#6366f1",
        desc: "Combines data from multiple branches. Supports 3 strategies: merge_objects (deep merge), array_concat (flatten arrays), and first_non_null (coalesce).",
      },
      {
        name: "Error Handler",
        color: "#f97316",
        desc: "Catches errors from upstream nodes. Placed anywhere reachable from the failing node. Returns a configurable fallback_value so execution can continue gracefully.",
      },
    ],
  },
  {
    category: "Actions",
    nodes: [
      {
        name: "HTTP Request",
        color: "#3b82f6",
        desc: "Make HTTP/HTTPS requests (GET, POST, PUT, DELETE, PATCH, HEAD). Validates URLs and blocks private IPs. Supports custom headers, body templates, per-node timeout, and retry. Requires network.http permission.",
      },
      {
        name: "LLM Request",
        color: "#a855f7",
        desc: "Call the user's configured LLM provider with a prompt template. Supports {{expression}} interpolation in prompts. Configurable max_tokens. Requires ai.inference permission.",
      },
      {
        name: "Channel Send",
        color: "#14b8a6",
        desc: "Send a message through any connected channel (Discord, Telegram, Slack, etc.). Template-expandable channel_id, recipient, and message. Requires channel.send permission.",
      },
      {
        name: "Native Tool",
        color: "#f43f5e",
        desc: "Execute any of the 29 built-in native tools by name (exec, read_file, web_search, git, etc.). Tool name and parameters support template expansion. Full access to the entire native tool suite.",
      },
      {
        name: "Sub-Flow",
        color: "#0ea5e9",
        desc: "Invoke a tool from another flowchart. Enables modular, reusable workflow composition. Recursion is prevented with a depth limit of 10.",
      },
      {
        name: "Storage Op",
        color: "#64748b",
        desc: "Read, write, or delete values from persistent extension storage. Template-expandable keys and values. No permission required.",
      },
      {
        name: "Config Get",
        color: "#78716c",
        desc: "Read user-set configuration values (stored with _config. prefix). Template-expandable key. No permission required.",
      },
    ],
  },
  {
    category: "Data",
    nodes: [
      {
        name: "Transform",
        color: "#ec4899",
        desc: "Transform data with 4 modes: json_path (extract values), template (interpolate strings), regex (match patterns with capture groups), and json_build (construct objects from templates).",
      },
      {
        name: "Set Variable",
        color: "#06b6d4",
        desc: "Assign a named variable for use in downstream nodes via $var.{name}. Value is evaluated from an expression. Useful for intermediate calculations and state.",
      },
      {
        name: "Output",
        color: "#ef4444",
        desc: "Terminal node that returns the final result. Supports result_template (with JSONPath or {{}} interpolation) or a literal result_value. If no template is set, merges all node outputs.",
      },
    ],
  },
  {
    category: "Utility",
    nodes: [
      {
        name: "Delay",
        color: "#a3a3a3",
        desc: "Pause execution for a specified duration. Capped at 30 seconds. Useful for rate limiting between API calls.",
      },
      {
        name: "Log",
        color: "#84cc16",
        desc: "Emit a debug log message at configurable levels (info, debug, warn, error). Message supports {{}} template interpolation.",
      },
      {
        name: "Comment",
        color: "#eab308",
        desc: "Annotation-only node for documentation. Does nothing at runtime. Rendered with a dashed border and sticky-note styling in the editor.",
      },
    ],
  },
];

const expressionOperators = [
  { op: "==", desc: "Equal (string, number, boolean comparison)" },
  { op: "!=", desc: "Not equal" },
  { op: ">", desc: "Greater than (numeric)" },
  { op: "<", desc: "Less than (numeric)" },
  { op: ">=", desc: "Greater than or equal" },
  { op: "<=", desc: "Less than or equal" },
  { op: "contains", desc: "String contains substring" },
  { op: "starts_with", desc: "String starts with prefix" },
  { op: "matches", desc: "String matches regex pattern" },
  { op: "exists", desc: "Value is not null (unary)" },
  { op: "is_null", desc: "Value is null (unary)" },
  { op: "is_string", desc: "Value is a string (unary)" },
  { op: "is_number", desc: "Value is a number (unary)" },
  { op: "is_array", desc: "Value is an array (unary)" },
  { op: "is_object", desc: "Value is an object (unary)" },
  { op: "&&", desc: "Logical AND" },
  { op: "||", desc: "Logical OR" },
  { op: "!", desc: "Logical NOT (prefix)" },
];

export default function FlowchartsPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Visual Automation
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Flowchart Builder
          </h1>
          <p className="text-muted-foreground mb-12">
            Build AI agent workflows visually. 19 node types, drag-and-drop
            editor, full access to native tools, LLM, and channels — no code
            required.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-1.5">
              {[
                { href: "#overview", label: "Overview" },
                { href: "#node-types", label: "Node Types (19)" },
                { href: "#expressions", label: "Expressions" },
                { href: "#editor", label: "Visual Editor" },
                { href: "#auto-triggers", label: "Auto-Triggers" },
                { href: "#permissions", label: "Permissions" },
                { href: "#testing", label: "Testing" },
                { href: "#definition-format", label: "Definition Format" },
              ].map((link) => (
                <a
                  key={link.href}
                  href={link.href}
                  className="text-[13px] text-muted-foreground hover:text-primary transition-colors px-2 py-1 rounded hover:bg-primary/5"
                >
                  {link.label}
                </a>
              ))}
            </div>
          </nav>

          {/* Overview */}
          <section className="mb-14" id="overview">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Overview
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              The Flowchart Builder lets you create complex AI agent workflows
              without writing code. Instead of building WASM extensions in Rust,
              you visually connect nodes in a drag-and-drop editor. Flowcharts
              can make HTTP requests, call LLMs, send messages through channels,
              execute any native tool, transform data, handle errors, and
              compose with other flowcharts via sub-flows.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Flowcharts are stored as JSON files and register their tools
              alongside native and extension tools. The LLM can call flowchart
              tools during conversations just like any other tool.
            </p>

            <div className="grid grid-cols-1 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { label: "Node Types", value: "19" },
                { label: "Expression Operators", value: "18" },
                { label: "Auto-Trigger Types", value: "3" },
                { label: "Max Execution", value: "500 nodes" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3 text-center">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">
                    {item.label}
                  </p>
                  <p className="text-sm font-medium">{item.value}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Node Types */}
          <section className="mb-14" id="node-types">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Node Types
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              19 node types organized into 4 categories. Each node has a
              specific purpose and configurable properties.
            </p>

            {nodeCategories.map((cat) => (
              <div key={cat.category} className="mb-10">
                <h3 className="text-sm font-medium text-foreground mb-4">
                  {cat.category} ({cat.nodes.length})
                </h3>
                <div className="space-y-4">
                  {cat.nodes.map((node) => (
                    <div
                      key={node.name}
                      className="border border-border/50 rounded-lg overflow-hidden"
                    >
                      <div className="bg-card/60 px-4 py-3 flex items-center gap-3">
                        <div
                          className="w-2.5 h-2.5 rounded-full shrink-0"
                          style={{ backgroundColor: node.color }}
                        />
                        <span className="text-sm font-medium">
                          {node.name}
                        </span>
                      </div>
                      <div className="bg-card px-4 py-3">
                        <p className="text-sm text-muted-foreground">
                          {node.desc}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </section>

          {/* Expressions */}
          <section className="mb-14" id="expressions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Expression System
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Flowcharts use a powerful expression system for accessing data,
              interpolating strings, and evaluating conditions. Three evaluation
              modes are available.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">
              Path Expressions (JSONPath)
            </h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Navigate through the execution context to access parameters, node
              outputs, and variables.
            </p>
            <div className="terminal mb-8">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">
                  path expressions
                </span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>
                  <span className="text-foreground/40">#</span> Access tool
                  parameters
                </p>
                <p>$.params.name</p>
                <p>$.params.user.email</p>
                <p>$.params.items[0]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Access node
                  outputs by ID
                </p>
                <p>$.nodes.http_1.body.data</p>
                <p>$.nodes.transform_1.result</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Access named
                  variables
                </p>
                <p>$var.counter</p>
                <p>$var.loop_item</p>
                <p>$var.loop_index</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Literals
                </p>
                <p>
                  &quot;hello&quot; &nbsp; 42 &nbsp; 3.14 &nbsp; true &nbsp;
                  false &nbsp; null
                </p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">
              Template Interpolation
            </h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Use{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">
                {"{{expression}}"}
              </code>{" "}
              to embed expressions inside strings. Templates are used in URL
              fields, message bodies, prompt templates, and more.
            </p>
            <div className="terminal mb-8">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">
                  templates
                </span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>
                  <span className="text-foreground/40">#</span> URL with
                  parameter interpolation
                </p>
                <p>
                  https://api.example.com/users/{"{{"}$.params.user_id{"}}"}
                </p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> LLM prompt with
                  node output
                </p>
                <p>
                  Summarize this data: {"{{"}$.nodes.http_1.body{"}}"}
                </p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Channel message
                </p>
                <p>
                  Alert: {"{{"}$.params.event_type{"}}"} from {"{{"}
                  $.params.source{"}}"}
                </p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">
              Condition Operators
            </h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Used in Condition and Switch nodes for boolean evaluation. Supports
              18 operators.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[auto_1fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Operator
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Description
                </div>
                {expressionOperators.map((row) => (
                  <>
                    <div
                      key={`o-${row.op}`}
                      className="bg-card px-3 py-2 text-sm font-mono text-primary/80"
                    >
                      {row.op}
                    </div>
                    <div
                      key={`d-${row.op}`}
                      className="bg-card px-3 py-2 text-sm text-muted-foreground"
                    >
                      {row.desc}
                    </div>
                  </>
                ))}
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">
              Condition Examples
            </h3>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">
                  conditions
                </span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>
                  <span className="text-foreground/40">#</span> Simple
                  comparison
                </p>
                <p>$.params.status == &quot;active&quot;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Numeric
                  comparison
                </p>
                <p>$.nodes.http_1.status &gt; 200</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> String contains
                </p>
                <p>$.params.message contains &quot;urgent&quot;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Boolean logic
                </p>
                <p>
                  $.params.role == &quot;admin&quot; && $.params.verified ==
                  true
                </p>
                <p>$.params.type == &quot;a&quot; || $.params.type == &quot;b&quot;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-foreground/40">#</span> Type checks
                </p>
                <p>$.params.data is_array</p>
                <p>$.params.name exists</p>
                <p>!$.params.deleted</p>
              </div>
            </div>
          </section>

          {/* Visual Editor */}
          <section className="mb-14" id="editor">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Visual Editor
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The flowchart editor is built with React Flow and provides a
              full-featured visual programming environment.
            </p>

            <div className="space-y-4 mb-8">
              {[
                {
                  feature: "Drag-and-Drop Canvas",
                  desc: "Drag nodes from the palette onto the canvas. Connect nodes by dragging from output handles to input handles. Pan and zoom with mouse or trackpad.",
                },
                {
                  feature: "Node Palette",
                  desc: "All 19 node types organized into 5 categories. Click or drag to add nodes to the canvas.",
                },
                {
                  feature: "Config Panel",
                  desc: "Select a node to configure it. Dynamic inputs based on node type with inline validation for URLs, JSON, and variable names.",
                },
                {
                  feature: "Expression Hints",
                  desc: "Collapsible panel on Condition, Switch, Transform, SetVariable, and Output nodes showing available expression patterns ($.params, $.nodes, $var).",
                },
                {
                  feature: "Auto Edge Labels",
                  desc: "Condition edges auto-label as \"True\"/\"False\". Switch edges label as \"Case 0\", \"Case 1\", \"Default\". Labels update as you connect nodes.",
                },
                {
                  feature: "Copy & Paste",
                  desc: "Select nodes and press Ctrl+C / Ctrl+V to duplicate them with new IDs. Pasted nodes are offset by 40px to avoid overlap.",
                },
                {
                  feature: "MiniMap",
                  desc: "Color-coded minimap showing all nodes. Each node type has a distinct color. Click to navigate to any area of the flowchart.",
                },
                {
                  feature: "Validation",
                  desc: "Real-time validation checks for missing nodes, disconnected graphs, invalid JSON config, and unreachable branches. Errors prevent saving; warnings are advisory.",
                },
              ].map((item) => (
                <div
                  key={item.feature}
                  className="border border-border/50 rounded-lg overflow-hidden"
                >
                  <div className="bg-card/60 px-4 py-3">
                    <span className="text-sm font-medium">{item.feature}</span>
                  </div>
                  <div className="bg-card px-4 py-3">
                    <p className="text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Engine */}
          <section className="mb-14" id="engine">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Execution Engine
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Flowcharts are executed by a native Rust engine that traverses the
              node graph asynchronously. The engine supports parallel branch
              execution, per-node timeouts, retries, and error handling.
            </p>

            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Limit
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Default
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Description
                </div>
                {[
                  {
                    limit: "Max Node Executions",
                    def: "500",
                    desc: "Maximum total node executions per run. Prevents infinite loops.",
                  },
                  {
                    limit: "Global Timeout",
                    def: "30s",
                    desc: "Maximum wall-clock time for a single flowchart execution.",
                  },
                  {
                    limit: "Max Delay",
                    def: "30s",
                    desc: "Maximum sleep duration for Delay nodes.",
                  },
                  {
                    limit: "Sub-Flow Depth",
                    def: "10",
                    desc: "Maximum recursion depth for nested sub-flow calls.",
                  },
                  {
                    limit: "Max Retries",
                    def: "10",
                    desc: "Maximum retry count per node (configurable per node).",
                  },
                  {
                    limit: "Loop Iterations",
                    def: "100",
                    desc: "Maximum iterations per Loop node (configurable per node).",
                  },
                ].map((row) => (
                  <>
                    <div
                      key={`l-${row.limit}`}
                      className="bg-card px-3 py-2 text-sm font-mono text-primary/80"
                    >
                      {row.limit}
                    </div>
                    <div
                      key={`d-${row.limit}`}
                      className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono"
                    >
                      {row.def}
                    </div>
                    <div
                      key={`e-${row.limit}`}
                      className="bg-card px-3 py-2 text-sm text-muted-foreground"
                    >
                      {row.desc}
                    </div>
                  </>
                ))}
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">
              Execution Flow
            </h3>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">
                  execution flow
                </span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>Tool call received (from LLM or auto-trigger)</p>
                <p>&nbsp; &darr;</p>
                <p>Resolve trigger node &rarr; store params in context</p>
                <p>&nbsp; &darr;</p>
                <p>Execute nodes along edges (sequential chains use loops)</p>
                <p>&nbsp; &darr;</p>
                <p>
                  Fan-out: multiple outgoing edges execute branches in parallel
                </p>
                <p>&nbsp; &darr;</p>
                <p>
                  Each action node: permission check &rarr; execute &rarr; store
                  output
                </p>
                <p>&nbsp; &darr;</p>
                <p>
                  On error: BFS search for reachable ErrorHandler &rarr; fallback
                </p>
                <p>&nbsp; &darr;</p>
                <p>Output node: evaluate result template &rarr; return value</p>
              </div>
            </div>
          </section>

          {/* Auto-Triggers */}
          <section className="mb-14" id="auto-triggers">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Auto-Triggers
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Flowcharts can be triggered automatically without a user or LLM
              initiating a tool call. Three trigger types are supported.
            </p>

            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                {
                  type: "Event",
                  desc: "Listens to the EventBus for specific event types (e.g., ChannelMessageReceived, UserJoined). Invokes the flowchart tool when a matching event fires, passing event data as parameters.",
                  config: "event_types: string[]",
                },
                {
                  type: "Schedule",
                  desc: "Executes at regular intervals using a configurable timer. Minimum interval is 5 seconds. Graceful shutdown on stop. Useful for periodic checks, cleanup, and monitoring.",
                  config: "interval_secs: number (min 5)",
                },
                {
                  type: "Webhook",
                  desc: "Registers an HTTP endpoint that triggers the flowchart when called. Request body JSON is passed as parameters. Supports GET, POST, PUT, DELETE methods.",
                  config: "path: string, method: string",
                },
              ].map((t) => (
                <div key={t.type} className="bg-card px-4 py-4">
                  <p className="text-sm font-medium text-primary/80 mb-2">
                    {t.type}
                  </p>
                  <p className="text-xs text-muted-foreground leading-relaxed mb-2">
                    {t.desc}
                  </p>
                  <p className="text-[11px] font-mono text-muted-foreground/60">
                    {t.config}
                  </p>
                </div>
              ))}
            </div>

            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">
                  auto-trigger config (in flowchart JSON)
                </span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>
                  <span className="text-foreground/60">
                    &quot;auto_triggers&quot;
                  </span>
                  : [
                </p>
                <p>&nbsp; {"{"}</p>
                <p>
                  &nbsp; &nbsp;{" "}
                  <span className="text-success">
                    &quot;id&quot;: &quot;at_1&quot;
                  </span>
                  ,
                </p>
                <p>
                  &nbsp; &nbsp;{" "}
                  <span className="text-success">
                    &quot;trigger_type&quot;: &quot;event&quot;
                  </span>
                  ,
                </p>
                <p>
                  &nbsp; &nbsp;{" "}
                  <span className="text-success">
                    &quot;tool_name&quot;: &quot;main&quot;
                  </span>
                  ,
                </p>
                <p>
                  &nbsp; &nbsp;{" "}
                  <span className="text-success">
                    &quot;config&quot;: {"{"} &quot;event_types&quot;:
                    [&quot;ChannelMessageReceived&quot;] {"}"}
                  </span>
                  ,
                </p>
                <p>
                  &nbsp; &nbsp;{" "}
                  <span className="text-success">
                    &quot;enabled&quot;: true
                  </span>
                </p>
                <p>&nbsp; {"}"}</p>
                <p>]</p>
              </div>
            </div>
          </section>

          {/* Permissions */}
          <section className="mb-14" id="permissions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Permissions
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Action nodes that access external resources are permission-gated.
              Permissions are declared in the flowchart definition and checked at
              runtime via the{" "}
              <Link
                href="/docs/security#permissions"
                className="text-primary hover:underline"
              >
                PolicyEngine
              </Link>
              .
            </p>

            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Node Type
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Required Capability
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Notes
                </div>
                {[
                  {
                    node: "HTTP Request",
                    cap: "network.http",
                    note: "URL validation blocks private/loopback IPs. Custom header and body support.",
                  },
                  {
                    node: "LLM Request",
                    cap: "ai.inference",
                    note: "Uses the user's configured LLM provider via LlmCallback bridge.",
                  },
                  {
                    node: "Channel Send",
                    cap: "channel.send",
                    note: "Sends through connected channel plugins via ChannelCallback bridge.",
                  },
                  {
                    node: "Native Tool",
                    cap: "(varies by tool)",
                    note: "Each native tool has its own required capability. Permission checked by NativeToolRegistry.",
                  },
                  {
                    node: "Sub-Flow",
                    cap: "(varies by target)",
                    note: "The target flowchart's permissions are checked independently.",
                  },
                  {
                    node: "Storage Op",
                    cap: "none",
                    note: "Extension-scoped storage. Always available.",
                  },
                  {
                    node: "Config Get",
                    cap: "none",
                    note: "Reads user-set config values. Always available.",
                  },
                ].map((row) => (
                  <>
                    <div
                      key={`n-${row.node}`}
                      className="bg-card px-3 py-2 text-sm text-foreground"
                    >
                      {row.node}
                    </div>
                    <div
                      key={`c-${row.node}`}
                      className="bg-card px-3 py-2 text-sm font-mono text-primary/80"
                    >
                      {row.cap}
                    </div>
                    <div
                      key={`d-${row.node}`}
                      className="bg-card px-3 py-2 text-sm text-muted-foreground"
                    >
                      {row.note}
                    </div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Testing */}
          <section className="mb-14" id="testing">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Testing & Debugging
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The editor includes a built-in test panel for running flowcharts
              with sample parameters and inspecting execution traces.
            </p>

            <div className="space-y-3 mb-6">
              {[
                "Select the tool to test from the dropdown (flowcharts can define multiple tools)",
                "Enter test parameters as JSON in the textarea",
                "Click Run — the engine executes with tracing enabled",
                "View the result (success output or error message) and execution time",
                "Expand the Node Trace section to see every node that executed, in order",
                "Each trace entry shows: sequence number, status (pass/fail), label, type, duration in ms, and any error message",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Definition Format */}
          <section className="mb-14" id="definition-format">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Definition Format
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Flowcharts are stored as JSON files in the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">
                flowcharts/
              </code>{" "}
              directory. Each file contains the full definition including nodes,
              edges, tools, permissions, and auto-triggers.
            </p>

            <div className="border border-border/50 rounded-lg overflow-hidden divide-y divide-border/50 mb-6">
              {[
                {
                  key: "id",
                  desc: "Unique identifier in format flow.user.{name}. Used for sub-flow references and storage.",
                },
                {
                  key: "name / version / author",
                  desc: "Human-readable metadata. Version follows semver.",
                },
                {
                  key: "enabled",
                  desc: "Whether this flowchart's tools are registered with the agent. Toggle from UI without deleting.",
                },
                {
                  key: "tools[]",
                  desc: "Array of tool definitions. Each has a name, description, JSON Schema parameters, and trigger_node_id pointing to the entry Trigger node.",
                },
                {
                  key: "permissions[]",
                  desc: "Array of capability declarations (capability, reason, required). Same format as extension manifests.",
                },
                {
                  key: "auto_triggers[]",
                  desc: "Array of auto-trigger definitions (event, schedule, or webhook). Each has its own enabled flag.",
                },
                {
                  key: "nodes[]",
                  desc: "Array of node objects with id, node_type, label, position ({x, y}), and config object (varies by type).",
                },
                {
                  key: "edges[]",
                  desc: "Array of edge objects with id, source, target, source_handle, target_handle, and optional label.",
                },
                {
                  key: "viewport",
                  desc: "Editor viewport state ({x, y, zoom}). Preserved when saving so you return to the same view.",
                },
              ].map((item) => (
                <div key={item.key} className="p-4 bg-card">
                  <code className="text-sm font-mono text-primary">{item.key}</code>
                  <p className="text-sm text-muted-foreground mt-1">{item.desc}</p>
                </div>
              ))}
            </div>

            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">
                  minimal flowchart example
                </span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>{"{"}</p>
                <p>
                  &nbsp; <span className="text-success">&quot;id&quot;</span>:{" "}
                  <span className="text-success">&quot;flow.user.greeting&quot;</span>,
                </p>
                <p>
                  &nbsp; <span className="text-success">&quot;name&quot;</span>:{" "}
                  <span className="text-success">&quot;Greeting Bot&quot;</span>,
                </p>
                <p>
                  &nbsp; <span className="text-success">&quot;enabled&quot;</span>:{" "}
                  <span className="text-warning">true</span>,
                </p>
                <p>
                  &nbsp; <span className="text-success">&quot;tools&quot;</span>: [{"{"}{" "}
                  <span className="text-success">&quot;name&quot;</span>:{" "}
                  <span className="text-success">&quot;greet&quot;</span>,{" "}
                  <span className="text-success">&quot;trigger_node_id&quot;</span>:{" "}
                  <span className="text-success">&quot;trigger_1&quot;</span>,{" "}
                  ... {"}"}],
                </p>
                <p>
                  &nbsp; <span className="text-success">&quot;nodes&quot;</span>: [
                </p>
                <p>
                  &nbsp; &nbsp; {"{"}{" "}
                  <span className="text-success">&quot;id&quot;</span>:{" "}
                  <span className="text-success">&quot;trigger_1&quot;</span>,{" "}
                  <span className="text-success">&quot;node_type&quot;</span>:{" "}
                  <span className="text-success">&quot;trigger&quot;</span>,{" "}
                  ... {"}"},
                </p>
                <p>
                  &nbsp; &nbsp; {"{"}{" "}
                  <span className="text-success">&quot;id&quot;</span>:{" "}
                  <span className="text-success">&quot;llm_1&quot;</span>,{" "}
                  <span className="text-success">&quot;node_type&quot;</span>:{" "}
                  <span className="text-success">&quot;llm_request&quot;</span>,{" "}
                  ... {"}"},
                </p>
                <p>
                  &nbsp; &nbsp; {"{"}{" "}
                  <span className="text-success">&quot;id&quot;</span>:{" "}
                  <span className="text-success">&quot;output_1&quot;</span>,{" "}
                  <span className="text-success">&quot;node_type&quot;</span>:{" "}
                  <span className="text-success">&quot;output&quot;</span>,{" "}
                  ... {"}"}
                </p>
                <p>&nbsp; ],</p>
                <p>
                  &nbsp; <span className="text-success">&quot;edges&quot;</span>: [
                </p>
                <p>
                  &nbsp; &nbsp; {"{"}{" "}
                  <span className="text-success">&quot;source&quot;</span>:{" "}
                  <span className="text-success">&quot;trigger_1&quot;</span>,{" "}
                  <span className="text-success">&quot;target&quot;</span>:{" "}
                  <span className="text-success">&quot;llm_1&quot;</span>{" "}
                  {"}"},
                </p>
                <p>
                  &nbsp; &nbsp; {"{"}{" "}
                  <span className="text-success">&quot;source&quot;</span>:{" "}
                  <span className="text-success">&quot;llm_1&quot;</span>,{" "}
                  <span className="text-success">&quot;target&quot;</span>:{" "}
                  <span className="text-success">&quot;output_1&quot;</span>{" "}
                  {"}"}
                </p>
                <p>&nbsp; ]</p>
                <p>{"}"}</p>
              </div>
            </div>
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Next Steps
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <Link
                href="/docs/tools"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Native Tools Reference
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  See all 29 tools available via the Native Tool node.
                </p>
              </Link>
              <Link
                href="/docs/hooks"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Hook System
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Intercept flowchart tool calls with hooks.
                </p>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
