import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Terms of Service — Usage & Legal Policies",
  description:
    "Review the Omni Marketplace terms of service covering account usage, WASM extension publishing rules, intellectual property, AI agent platform policies, and developer community guidelines.",
  openGraph: {
    title: "Terms of Service — Omni Marketplace Usage & Legal Policies",
    description:
      "Review the Omni Marketplace terms of service covering account usage, WASM extension publishing rules, intellectual property, AI agent platform policies, and community guidelines.",
    url: "/terms",
  },
  alternates: { canonical: "/terms" },
};

const sections = [
  {
    title: "1. Acceptance of Terms",
    text: "By creating an account or using the Service, you acknowledge that you have read, understood, and agree to be bound by these Terms and our Privacy Policy. If you do not agree, you may not use the Service. You must be at least 13 years old to use the Service. If you are between 13 and 18, you must have parental or guardian consent.",
  },
  {
    title: "2. Account Registration",
    text: "To publish extensions or leave reviews, you must create an account. You agree to provide accurate, current, and complete registration information; maintain the security of your password and API keys; promptly update your information if it changes; accept responsibility for all activity under your account; and not share your account credentials or API keys with unauthorized parties. We reserve the right to suspend or terminate accounts that violate these Terms or that we reasonably believe are engaged in fraudulent or harmful activity.",
  },
  {
    title: "3. Extension Publishing",
    text: "By publishing an extension to the Omni Marketplace, you represent and warrant that you own the extension or have the right to publish it, the extension does not infringe any third-party intellectual property rights, the extension's description accurately represents its functionality, the extension does not contain malware, spyware, or malicious code, and the extension complies with all applicable laws and regulations. Extensions must not collect or transmit user data without disclosure, attempt to escalate permissions or escape the WASM sandbox, contain hidden functionality not described in the manifest, impersonate other extensions or developers, distribute copyrighted content without authorization, facilitate illegal activities, mine cryptocurrency without consent, or interfere with the Omni platform. You retain all intellectual property rights to extensions you publish. By publishing, you grant Omni a non-exclusive, worldwide license to host, distribute, display, and scan your extension for operating the marketplace. This license terminates when you unpublish or delete your extension.",
  },
  {
    title: "4. Marketplace Usage",
    text: "As a user, you agree to review extension permissions before installation and make informed decisions; not reverse-engineer, decompile, or modify extensions in violation of their licenses; not use automated tools to scrape, crawl, or bulk-download extensions; leave honest and constructive reviews based on genuine experience; and report security vulnerabilities through our responsible disclosure process.",
  },
  {
    title: "5. Security Scanning",
    text: "By publishing an extension, you consent to our automated security scanning process. All submitted WASM binaries and manifests are analyzed by our 4-layer antivirus pipeline, which includes AI-powered code review. We reserve the right to reject, remove, or flag any extension that fails our security scan or that we reasonably believe poses a risk to users, regardless of the automated scan result. Scan results are final for automated decisions; flagged extensions may be appealed through manual review.",
  },
  {
    title: "6. Intellectual Property",
    text: "The Omni name, logo, website design, and marketplace infrastructure are the intellectual property of Omni. The Omni desktop application is open-source software distributed under its respective license. Third-party extensions are the intellectual property of their respective publishers and are distributed under their stated licenses.",
  },
  {
    title: "7. Prohibited Conduct",
    text: "You agree not to use the Service for any unlawful purpose; attempt to gain unauthorized access to the Service or its infrastructure; interfere with or disrupt the Service or servers; submit false or misleading information; create multiple accounts to circumvent restrictions or bans; use the Service to distribute spam, malware, or phishing content; manipulate extension ratings, reviews, or download counts; or violate the rights of other users or third parties.",
  },
  {
    title: "8. Disclaimers",
    text: "THE SERVICE IS PROVIDED \"AS IS\" AND \"AS AVAILABLE\" WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, AND NON-INFRINGEMENT. While we scan all extensions for security threats, we do not guarantee that extensions are free from all vulnerabilities, bugs, or harmful behavior. Users install extensions at their own discretion and should review permissions carefully. We do not guarantee the availability, reliability, or performance of the Service.",
  },
  {
    title: "9. Limitation of Liability",
    text: "TO THE MAXIMUM EXTENT PERMITTED BY LAW, OMNI SHALL NOT BE LIABLE FOR ANY INDIRECT, INCIDENTAL, SPECIAL, CONSEQUENTIAL, OR PUNITIVE DAMAGES, INCLUDING BUT NOT LIMITED TO LOSS OF DATA, LOSS OF REVENUE, OR LOSS OF BUSINESS OPPORTUNITIES, ARISING FROM YOUR USE OF THE SERVICE OR ANY EXTENSION INSTALLED THROUGH THE MARKETPLACE. OUR TOTAL LIABILITY SHALL NOT EXCEED THE AMOUNT YOU PAID TO US IN THE TWELVE MONTHS PRECEDING THE CLAIM, OR $100, WHICHEVER IS GREATER.",
  },
  {
    title: "10. Termination",
    text: "We may suspend or terminate your access to the Service at any time, with or without cause, with or without notice. Reasons for termination include violation of these Terms, publishing malicious or prohibited extensions, fraudulent activity or abuse of the platform, and extended periods of inactivity. Upon termination, your right to use the Service ceases immediately. Published extensions will be unpublished. You may request a copy of your data for 30 days following termination.",
  },
  {
    title: "11. Modifications to Terms",
    text: "We may modify these Terms at any time. We will notify registered users of material changes via email or through a prominent notice on the Service at least 30 days before the changes take effect. Your continued use of the Service after the effective date constitutes acceptance of the modified Terms.",
  },
  {
    title: "12. Governing Law",
    text: "These Terms shall be governed by and construed in accordance with the laws of the jurisdiction in which Omni operates, without regard to its conflict of law provisions. Any disputes arising from these Terms shall be resolved through binding arbitration in accordance with applicable arbitration rules, except where prohibited by law.",
  },
  {
    title: "13. Contact",
    text: "If you have questions about these Terms of Service, please contact us at legal@omniapp.org.",
  },
];

export default function TermsOfServicePage() {
  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <p className="text-sm font-medium text-primary mb-3">Legal</p>
      <h1 className="text-3xl font-bold tracking-tight mb-2">Terms of Service</h1>
      <p className="text-xs font-mono text-muted-foreground mb-12">Last updated: February 15, 2026</p>

      <p className="text-muted-foreground leading-relaxed mb-12">
        These Terms of Service (&ldquo;Terms&rdquo;) govern your use of the Omni Marketplace
        website, the Omni desktop application, and related services (collectively, the &ldquo;Service&rdquo;)
        operated by Omni. By accessing or using the Service, you agree to be bound by these Terms.
      </p>

      <div className="space-y-10">
        {sections.map((section) => (
          <section key={section.title}>
            <h2 className="text-lg font-medium mb-4 pb-2 border-b border-border/50">
              {section.title}
            </h2>
            <p className="text-sm text-muted-foreground leading-relaxed">{section.text}</p>
          </section>
        ))}
      </div>
    </div>
  );
}
