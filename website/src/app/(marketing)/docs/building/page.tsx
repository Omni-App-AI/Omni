import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Building from Source — Compile Omni Locally",
  description:
    "Build the Omni AI agent from source on Windows, macOS, or Linux. Step-by-step Rust toolchain setup, dependency installation, WASM runtime compilation, and development workflow guide.",
  openGraph: {
    title: "Build Omni from Source — Rust Compilation & Dev Setup",
    description:
      "Build the Omni AI agent from source on Windows, macOS, or Linux. Rust toolchain setup, dependency installation, WASM runtime compilation, and development workflow.",
    url: "/docs/building",
  },
  alternates: { canonical: "/docs/building" },
};

const prerequisites = [
  {
    name: "Rust toolchain",
    version: "1.78+",
    install: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
  },
  {
    name: "wasm32-wasi target",
    version: "",
    install: "rustup target add wasm32-wasip1",
  },
  {
    name: "Node.js",
    version: "20+",
    install: "Required for the WhatsApp Baileys sidecar",
  },
  {
    name: "SQLite dev headers",
    version: "3.35+",
    install: "apt install libsqlite3-dev  (Linux) / brew install sqlite  (macOS)",
  },
  {
    name: "OpenSSL dev headers",
    version: "1.1+",
    install: "apt install libssl-dev pkg-config  (Linux) / brew install openssl  (macOS)",
  },
];

const crates = [
  { name: "omni-core", desc: "Agent loop, event bus, configuration, and database layer" },
  { name: "omni-channels", desc: "All 21+ messaging platform integrations" },
  { name: "omni-runtime", desc: "WASM extension runtime with Wasmtime, host functions, and sandbox enforcement" },
  { name: "omni-guardian", desc: "Anti-injection pipeline and capability-based permission system" },
  { name: "omni-tools", desc: "29 built-in native tools (exec, file, web, memory, git, testing, debugging, REPL, etc.)" },
  { name: "omni-cli", desc: "CLI binary, extension publishing commands, and config migration" },
  { name: "omni-sdk", desc: "Guest-side SDK crate for building WASM extensions" },
];

export default function BuildingFromSourcePage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-3xl">
          <p className="text-sm font-medium text-primary mb-3">Docs</p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">Building from Source</h1>
          <p className="text-muted-foreground mb-12">
            Compile the Omni agent, runtime, and CLI from source. Useful for contributors,
            custom builds, and running on architectures without pre-built binaries.
          </p>

          {/* Prerequisites */}
          <section className="mb-14" id="prerequisites">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Prerequisites</h2>
            <div className="space-y-0 border border-border/50 rounded-lg overflow-hidden">
              {prerequisites.map((dep, i) => (
                <div
                  key={dep.name}
                  className={`bg-card p-4 ${i > 0 ? "border-t border-border/50" : ""}`}
                >
                  <div className="flex items-baseline gap-2 mb-1">
                    <span className="font-medium text-sm">{dep.name}</span>
                    {dep.version && (
                      <code className="text-xs font-mono text-muted-foreground">{dep.version}</code>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground font-mono">{dep.install}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Clone & Build */}
          <section className="mb-14" id="clone-build">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Clone &amp; Build</h2>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Clone the repository</p>
                <p>git clone https://github.com/omniai/omni.git</p>
                <p>cd omni</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Build all crates in release mode</p>
                <p>cargo build --release</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Binary location</p>
                <p>./target/release/omni <span className="text-foreground/40">--version</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              A full release build takes 3-5 minutes on a modern machine. Debug builds are faster
              but produce significantly larger binaries.
            </p>
          </section>

          {/* Crate Structure */}
          <section className="mb-14" id="crate-structure">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Crate Structure</h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Omni is a Cargo workspace with seven crates. Each can be built and tested independently.
            </p>
            <div className="space-y-4">
              {crates.map((crate, i) => (
                <div key={crate.name} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <code className="font-medium text-[15px] font-mono">{crate.name}</code>
                    <p className="mt-1 text-sm text-muted-foreground">{crate.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Platform-Specific Notes */}
          <section className="mb-14" id="platform-notes">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Platform-Specific Notes</h2>

            <h3 className="text-sm font-medium text-foreground mb-3 mt-6">Windows</h3>
            <ul className="space-y-2 text-sm text-muted-foreground mb-6">
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                Install Visual Studio Build Tools 2022 with the &ldquo;Desktop development with C++&rdquo; workload
              </li>
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                Use <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">vcpkg</code> to install OpenSSL:{" "}
                <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">vcpkg install openssl:x64-windows</code>
              </li>
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                Set <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">OPENSSL_DIR</code> and{" "}
                <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">OPENSSL_LIB_DIR</code> environment variables
              </li>
            </ul>

            <h3 className="text-sm font-medium text-foreground mb-3">macOS</h3>
            <ul className="space-y-2 text-sm text-muted-foreground mb-6">
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                Xcode Command Line Tools required:{" "}
                <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">xcode-select --install</code>
              </li>
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                Apple Silicon (M1/M2/M3) is natively supported — builds produce arm64 binaries
              </li>
            </ul>

            <h3 className="text-sm font-medium text-foreground mb-3">Linux</h3>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">apt (Debian/Ubuntu)</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p>sudo apt update</p>
                <p>sudo apt install build-essential pkg-config \</p>
                <p>  libssl-dev libsqlite3-dev</p>
              </div>
            </div>
          </section>

          {/* Running Tests */}
          <section className="mb-14" id="tests">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Running Tests</h2>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">cargo test</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Run the full test suite</p>
                <p>cargo test --workspace</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Run tests for a specific crate</p>
                <p>cargo test -p omni-runtime</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Run with logging output</p>
                <p>RUST_LOG=debug cargo test --workspace -- --nocapture</p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Integration tests for channel crates require environment variables for bot tokens.
              See <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">.env.example</code> for
              the full list.
            </p>
          </section>

          {/* Development Workflow */}
          <section className="mb-14" id="dev-workflow">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Development Workflow</h2>
            <div className="space-y-4">
              {[
                { title: "Watch mode", desc: "Use cargo watch -x run to auto-rebuild on file changes. Install with cargo install cargo-watch." },
                { title: "Check before committing", desc: "Run cargo clippy --workspace -- -D warnings and cargo fmt --check to match CI lint rules." },
                { title: "Build extensions locally", desc: "Use cargo build --target wasm32-wasip1 --release in your extension crate, then omni ext install --path ./target/..." },
                { title: "Debug logging", desc: "Set RUST_LOG=omni_core=debug,omni_runtime=trace for granular runtime output." },
              ].map((item, i) => (
                <div key={item.title} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <h4 className="font-medium text-[15px]">{item.title}</h4>
                    <p className="mt-1 text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Next Steps</h2>
            <div className="grid sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { href: "/docs/architecture", title: "Architecture", desc: "Understand the crate structure and data flow" },
                { href: "/docs/sdk", title: "SDK Reference", desc: "Build your first WASM extension" },
                { href: "/docs/configuration", title: "Configuration", desc: "Set up providers, channels, and permissions" },
                { href: "/docs/changelog", title: "Changelog", desc: "See what changed in each release" },
              ].map((link) => (
                <Link
                  key={link.href}
                  href={link.href}
                  className="bg-card p-5 hover:bg-secondary/50 transition-colors"
                >
                  <h3 className="font-medium text-sm mb-1">{link.title}</h3>
                  <p className="text-xs text-muted-foreground">{link.desc}</p>
                </Link>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
