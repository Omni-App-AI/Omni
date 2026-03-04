import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Hook System — Agent Loop Interception",
  description:
    "Intercept and modify data at 7 points in the Omni AI agent loop. Configure WASM hooks for message filtering, tool gating, output transformation, and session lifecycle management.",
  openGraph: {
    title: "Omni Hook System — AI Agent Loop Interception Points",
    description:
      "Intercept and modify data at 7 points in the Omni AI agent loop. Configure WASM hooks for message filtering, tool gating, output transformation, and session lifecycle.",
    url: "/docs/hooks",
  },
  alternates: { canonical: "/docs/hooks" },
};

export default function HooksPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Advanced
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Hook System
          </h1>
          <p className="text-muted-foreground mb-12">
            Intercept and modify data at 7 points in the agent loop. Block tool calls,
            transform messages, and react to session events.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#overview", label: "Overview" },
                { href: "#hook-points", label: "Hook Points" },
                { href: "#modifying-hooks", label: "Modifying Hooks" },
                { href: "#notification-hooks", label: "Notification Hooks" },
                { href: "#hook-context", label: "Hook Context" },
                { href: "#hook-results", label: "Hook Results" },
                { href: "#registration", label: "Registration" },
                { href: "#examples", label: "Examples" },
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
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Overview</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              The hook system lets you intercept data at key points in the agent loop. Hooks can
              inspect, modify, or block data as it flows through the system. This enables use
              cases like content filtering, logging, rate limiting, and custom security policies.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-6">
              There are two types of hooks:
            </p>
            <div className="grid grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <div className="bg-card px-4 py-4">
                <p className="text-sm font-medium mb-1">Modifying Hooks</p>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  Run sequentially in priority order. Can transform data or block it entirely.
                  If one hook blocks, the pipeline stops and downstream hooks don&apos;t run.
                </p>
              </div>
              <div className="bg-card px-4 py-4">
                <p className="text-sm font-medium mb-1">Notification Hooks</p>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  Run in parallel. Fire-and-forget for observability. Cannot modify data or
                  block the pipeline. Used for logging, analytics, and external integrations.
                </p>
              </div>
            </div>
          </section>

          {/* Hook Points */}
          <section className="mb-14" id="hook-points">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Hook Points
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Omni provides 7 hook points — 5 modifying and 2 notification.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_auto_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Hook Point</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { point: "MessageReceived", type: "Modifying", desc: "Fired when a user message arrives. Can modify the message text or block it before it enters the conversation." },
                  { point: "LlmInput", type: "Modifying", desc: "Fired before the assembled prompt is sent to the LLM. Can modify messages, add context, or block the request." },
                  { point: "LlmOutput", type: "Modifying", desc: "Fired after the LLM responds. Can transform the response text or block it before tool processing begins." },
                  { point: "BeforeToolCall", type: "Modifying", desc: "Fired before a tool is executed. Can modify parameters or block specific tool calls." },
                  { point: "AfterToolCall", type: "Modifying", desc: "Fired after a tool returns. Can modify the tool's result before it's fed back to the LLM." },
                  { point: "SessionStart", type: "Notification", desc: "Fired when a new session begins. Use for initialization, logging, or external notifications." },
                  { point: "SessionEnd", type: "Notification", desc: "Fired when a session ends. Use for cleanup, analytics, or summary generation." },
                ].map((row) => (
                  <>
                    <div key={`p-${row.point}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.point}</div>
                    <div key={`t-${row.point}`} className="bg-card px-3 py-2 text-xs text-muted-foreground">
                      <span className={row.type === "Modifying" ? "text-warning" : "text-success"}>
                        {row.type}
                      </span>
                    </div>
                    <div key={`d-${row.point}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Modifying Hooks */}
          <section className="mb-14" id="modifying-hooks">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Modifying Hooks
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Modifying hooks run sequentially in priority order (lowest number first). Each hook
              receives the output of the previous hook. A hook can return{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Continue</code>{" "}
              (optionally with modified data) or{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Block</code>{" "}
              to stop the pipeline.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">execution order</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>Hook A (priority 10) &rarr; Continue(modified data)</p>
                <p>  &darr;</p>
                <p>Hook B (priority 20) &rarr; Continue(data)</p>
                <p>  &darr;</p>
                <p>Hook C (priority 30) &rarr; Block(&quot;reason&quot;)</p>
                <p>  &darr;</p>
                <p>Pipeline stops. Hook D (priority 40) never runs.</p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              When a BeforeToolCall hook blocks, the agent loop receives a{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">HookBlocked</code> error
              and skips the tool execution. The LLM is informed that the tool call was blocked.
            </p>
          </section>

          {/* Notification Hooks */}
          <section className="mb-14" id="notification-hooks">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Notification Hooks
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Notification hooks run in parallel using{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">tokio::join!</code>.
              They receive a read-only copy of the hook context. Errors in notification hooks
              are logged but don&apos;t affect the pipeline.
            </p>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Common use cases: sending analytics events, writing to external log services,
              triggering webhooks, or updating a dashboard when sessions start or end.
            </p>
          </section>

          {/* Hook Context */}
          <section className="mb-14" id="hook-context">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Hook Context
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Every hook receives a context object containing relevant data for the hook point.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_auto_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Field</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { field: "hook_point", type: "HookPoint", desc: "Which hook point fired this context." },
                  { field: "session_id", type: "Option<String>", desc: "Current session ID, if available." },
                  { field: "text", type: "Option<String>", desc: "Message text (for MessageReceived, LlmInput, LlmOutput)." },
                  { field: "tool_call", type: "Option<ToolCallInfo>", desc: "Tool name and arguments (for BeforeToolCall, AfterToolCall)." },
                  { field: "messages", type: "Option<Vec<ChatMessage>>", desc: "Full conversation history (for LlmInput)." },
                  { field: "metadata", type: "serde_json::Value", desc: "Arbitrary JSON metadata for custom data passing between hooks." },
                ].map((row) => (
                  <>
                    <div key={`f-${row.field}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.field}</div>
                    <div key={`t-${row.field}`} className="bg-card px-3 py-2 text-xs font-mono text-muted-foreground">{row.type}</div>
                    <div key={`d-${row.field}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Hook Results */}
          <section className="mb-14" id="hook-results">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Hook Results
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Modifying hooks return one of two results:
            </p>
            <div className="grid grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <div className="bg-card px-4 py-4">
                <p className="text-sm font-mono font-medium text-success mb-2">Continue(HookContext)</p>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  Let the pipeline proceed. Pass the context unchanged or with modifications.
                  The next hook in the chain receives the returned context.
                </p>
              </div>
              <div className="bg-card px-4 py-4">
                <p className="text-sm font-mono font-medium text-destructive mb-2">{"Block { reason: String }"}</p>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  Stop the pipeline. The reason string is logged and, for BeforeToolCall hooks,
                  returned to the LLM as a HookBlocked error so it can adapt.
                </p>
              </div>
            </div>
          </section>

          {/* Registration */}
          <section className="mb-14" id="registration">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Registration
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Hooks are registered with the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">HookRegistry</code>{" "}
              which is shared across the agent loop via{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Arc&lt;HookRegistry&gt;</code>.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">Rust</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p><span className="text-primary/70">let</span> registry = HookRegistry::new();</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">//</span> Register a modifying hook with priority 10</p>
                <p>registry.register_modifying(</p>
                <p>    HookPoint::BeforeToolCall,</p>
                <p>    <span className="text-warning">10</span>,  <span className="text-foreground/40">// priority</span></p>
                <p>    my_hook_handler,</p>
                <p>);</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">//</span> Register a notification hook</p>
                <p>registry.register_notification(</p>
                <p>    HookPoint::SessionStart,</p>
                <p>    my_notification_handler,</p>
                <p>);</p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Hook handlers implement the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">HookHandler</code> trait,
              which has a single async method that receives a{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">HookContext</code> and
              returns a <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">HookResult</code>.
            </p>
          </section>

          {/* Examples */}
          <section className="mb-14" id="examples">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Examples
            </h2>

            <h3 className="text-sm font-medium text-foreground mb-3">Block dangerous tool calls</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              A BeforeToolCall hook that prevents the agent from executing{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">exec</code> with
              destructive commands.
            </p>
            <div className="terminal mb-8">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">Rust</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p><span className="text-primary/70">async fn</span> handle(&amp;<span className="text-primary/70">self</span>, ctx: HookContext) -&gt; HookResult {"{"}</p>
                <p>    <span className="text-primary/70">if let</span> Some(tool_call) = &amp;ctx.tool_call {"{"}</p>
                <p>        <span className="text-primary/70">if</span> tool_call.name == <span className="text-success">&quot;exec&quot;</span> {"{"}</p>
                <p>            <span className="text-primary/70">let</span> cmd = tool_call.arguments</p>
                <p>                .get(<span className="text-success">&quot;command&quot;</span>)</p>
                <p>                .and_then(|v| v.as_str())</p>
                <p>                .unwrap_or_default();</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>            <span className="text-primary/70">let</span> blocked = [<span className="text-success">&quot;rm -rf&quot;</span>, <span className="text-success">&quot;format&quot;</span>, <span className="text-success">&quot;del /f&quot;</span>];</p>
                <p>            <span className="text-primary/70">if</span> blocked.iter().any(|b| cmd.contains(b)) {"{"}</p>
                <p>                <span className="text-primary/70">return</span> HookResult::Block {"{"}</p>
                <p>                    reason: <span className="text-success">&quot;Destructive command blocked&quot;</span>.into(),</p>
                <p>                {"}"};</p>
                <p>            {"}"}</p>
                <p>        {"}"}</p>
                <p>    {"}"}</p>
                <p>    HookResult::Continue(ctx)</p>
                <p>{"}"}</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Log all LLM requests</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              An LlmInput hook that logs the prompt being sent to the LLM for debugging.
            </p>
            <div className="terminal mb-8">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">Rust</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p><span className="text-primary/70">async fn</span> handle(&amp;<span className="text-primary/70">self</span>, ctx: HookContext) -&gt; HookResult {"{"}</p>
                <p>    <span className="text-primary/70">if let</span> Some(messages) = &amp;ctx.messages {"{"}</p>
                <p>        tracing::debug!(</p>
                <p>            <span className="text-success">&quot;LLM request: {"{}"} messages, session {"{{:?}}"}&quot;</span>,</p>
                <p>            messages.len(),</p>
                <p>            ctx.session_id,</p>
                <p>        );</p>
                <p>    {"}"}</p>
                <p>    HookResult::Continue(ctx)</p>
                <p>{"}"}</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Filter profanity from messages</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              A MessageReceived hook that redacts profanity from user messages.
            </p>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">Rust</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p><span className="text-primary/70">async fn</span> handle(&amp;<span className="text-primary/70">self</span>, <span className="text-primary/70">mut</span> ctx: HookContext) -&gt; HookResult {"{"}</p>
                <p>    <span className="text-primary/70">if let</span> Some(<span className="text-primary/70">ref mut</span> text) = ctx.text {"{"}</p>
                <p>        *text = self.profanity_filter.censor(text);</p>
                <p>    {"}"}</p>
                <p>    HookResult::Continue(ctx)</p>
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
                href="/docs/security"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Security & Permissions
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Learn about the Guardian pipeline and capability system.
                </p>
              </Link>
              <Link
                href="/docs/architecture#agent-loop"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Agent Loop Architecture
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  See where hooks fit in the full agent loop flow.
                </p>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
