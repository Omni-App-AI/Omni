const requirements = [
  {
    platform: "Windows",
    items: [
      "Windows 10 version 1803 or later",
      "x86_64 architecture",
      "WebView2 runtime (included in Windows 11)",
      "4 GB RAM minimum, 8 GB recommended",
    ],
  },
  {
    platform: "macOS",
    items: [
      "macOS 10.15 (Catalina) or later",
      "Intel x86_64 or Apple Silicon (ARM64)",
      "4 GB RAM minimum, 8 GB recommended",
    ],
  },
  {
    platform: "Linux",
    items: [
      "Ubuntu 20.04, Fedora 36, or equivalent",
      "x86_64 architecture",
      "WebKitGTK 4.1, libappindicator3, librsvg2",
      "4 GB RAM minimum, 8 GB recommended",
    ],
  },
];

export function SystemRequirements() {
  return (
    <div className="grid sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
      {requirements.map((req) => (
        <div key={req.platform} className="bg-card p-6">
          <h3 className="font-medium text-[15px] mb-3">{req.platform}</h3>
          <div className="space-y-2">
            {req.items.map((item, i) => (
              <p
                key={i}
                className="text-sm text-muted-foreground leading-relaxed"
              >
                {item}
              </p>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
