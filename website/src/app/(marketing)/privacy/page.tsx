import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Privacy Policy — Data Protection & Usage",
  description:
    "Read the Omni Marketplace privacy policy. Learn how we collect, use, store, and protect your personal data when using our AI agent platform, WASM extension marketplace, and developer tools.",
  openGraph: {
    title: "Privacy Policy — Omni Marketplace Data Protection & Usage",
    description:
      "Read the Omni Marketplace privacy policy. Learn how we collect, use, store, and protect your personal data when using our AI agent platform and extension marketplace.",
    url: "/privacy",
  },
  alternates: { canonical: "/privacy" },
};

const sections: { title: string; content: { heading?: string; text: string }[] }[] = [
  {
    title: "1. Information We Collect",
    content: [
      {
        heading: "Account Information",
        text: "When you create an account, we collect your email address, username, and display name. If you sign up via GitHub or Google OAuth, we receive your public profile information (name, email, avatar URL, and GitHub username if applicable) from the OAuth provider.",
      },
      {
        heading: "Extension Publishing Data",
        text: "When you publish an extension, we store your extension's metadata (name, description, version, permissions, tools), the compiled WASM binary, and your publisher profile information. This data is publicly visible on the marketplace.",
      },
      {
        heading: "Usage Data",
        text: "We collect anonymized download counts and aggregate analytics (extension popularity, category trends). We do not track individual user behavior, browsing patterns, or search queries. Download tracking uses hashed IP addresses for deduplication — we never store raw IP addresses.",
      },
    ],
  },
  {
    title: "2. How We Use Information",
    content: [
      {
        text: "We use the information we collect to: provide, maintain, and improve the Omni Marketplace; authenticate your identity and manage your account; process extension submissions and run security scans; display publisher profiles and extension listings; generate aggregate statistics (download counts, ratings); send important account notifications (scan results, policy changes); and detect and prevent fraud, abuse, or security threats.",
      },
    ],
  },
  {
    title: "3. Extension Scanning",
    content: [
      {
        text: "When you submit an extension, our automated security pipeline analyzes the WASM binary, manifest, and metadata. This includes pattern matching against known malicious signatures, heuristic analysis of permission requests and behavior, AI-powered code analysis using Anthropic's Claude API, and sandboxed execution testing in an isolated environment.",
      },
      {
        text: "Scan results (scores, findings, verdicts) are stored and may be publicly visible on the extension's detail page. The AI analysis component sends your extension's manifest and extracted metadata to Anthropic's API for processing. No user data is included in these requests — only the extension's code and metadata.",
      },
    ],
  },
  {
    title: "4. Data Storage & Security",
    content: [
      {
        text: "Your data is stored securely using Supabase (PostgreSQL) with row-level security policies. WASM binaries are stored in Supabase Storage with access controls. API keys are stored as SHA-256 hashes — we never store your raw API key. All data transmission uses HTTPS encryption.",
      },
    ],
  },
  {
    title: "5. Cookies",
    content: [
      {
        text: "We use essential cookies for authentication session management (Supabase auth tokens). These cookies are necessary for the Service to function and cannot be disabled. We do not use advertising cookies, tracking cookies, or third-party analytics cookies.",
      },
    ],
  },
  {
    title: "6. Third-Party Services",
    content: [
      {
        text: "We use the following third-party services: Supabase (database, authentication, file storage), Vercel (website hosting and deployment), GitHub and Google (OAuth authentication), and Anthropic (AI-powered extension scanning via Claude API). Each third-party service has its own privacy policy. We only share the minimum data necessary for each service to function.",
      },
    ],
  },
  {
    title: "7. Data Retention",
    content: [
      {
        text: "Account data is retained for as long as your account is active. Extension data (metadata, WASM binaries, scan results) is retained for as long as the extension is published. Download statistics are retained in aggregate form indefinitely. When you delete your account, your profile data and API keys are permanently removed. Published extensions are unpublished but may be retained for a grace period of 30 days.",
      },
    ],
  },
  {
    title: "8. Your Rights",
    content: [
      {
        text: "You have the right to: access and request a copy of all personal data we hold about you; correct inaccurate personal data; request deletion of your account and associated data; download your data in a machine-readable format; and object to specific uses of your data. To exercise any of these rights, contact us at privacy@omniapp.org. We will respond within 30 days.",
      },
    ],
  },
  {
    title: "9. The Omni Desktop Application",
    content: [
      {
        text: "The Omni desktop application runs entirely on your local machine. Conversations, LLM API keys, channel credentials, and extension data are stored locally and never transmitted to our servers. The marketplace website only receives data when you explicitly publish an extension, download an extension, or interact with the website. We do not collect telemetry, usage analytics, or crash reports from the desktop application.",
      },
    ],
  },
  {
    title: "10. Children's Privacy",
    content: [
      {
        text: "The Service is not intended for users under the age of 13. We do not knowingly collect personal information from children under 13. If we become aware that we have collected data from a child under 13, we will promptly delete that information.",
      },
    ],
  },
  {
    title: "11. Changes to This Policy",
    content: [
      {
        text: "We may update this Privacy Policy from time to time. We will notify registered users of material changes via email or through a notice on the Service. Your continued use of the Service after changes take effect constitutes acceptance of the updated policy.",
      },
    ],
  },
  {
    title: "12. Contact",
    content: [
      {
        text: "If you have questions about this Privacy Policy or our data practices, please contact us at privacy@omniapp.org.",
      },
    ],
  },
];

export default function PrivacyPolicyPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <p className="text-sm font-medium text-primary mb-3">Legal</p>
      <h1 className="text-3xl font-bold tracking-tight mb-2">Privacy Policy</h1>
      <p className="text-xs font-mono text-muted-foreground mb-12">Last updated: February 15, 2026</p>

      <p className="text-muted-foreground leading-relaxed mb-12">
        This Privacy Policy describes how Omni (&ldquo;we,&rdquo; &ldquo;our,&rdquo; or &ldquo;us&rdquo;)
        collects, uses, and shares information when you use the Omni Marketplace website,
        the Omni desktop application, and related services. We are committed to protecting
        your privacy and being transparent about our data practices.
      </p>

      <div className="space-y-10">
        {sections.map((section) => (
          <section key={section.title}>
            <h2 className="text-lg font-medium mb-4 pb-2 border-b border-border/50">
              {section.title}
            </h2>
            <div className="space-y-4">
              {section.content.map((block, i) => (
                <div key={i}>
                  {block.heading && (
                    <h3 className="text-sm font-medium text-foreground mb-2">{block.heading}</h3>
                  )}
                  <p className="text-sm text-muted-foreground leading-relaxed">{block.text}</p>
                </div>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
