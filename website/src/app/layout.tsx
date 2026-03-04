import type { Metadata } from "next";
import Script from "next/script";
import { Toaster } from "sonner";
import { Navbar } from "@/components/layout/Navbar";
import { Footer } from "@/components/layout/Footer";
import { createClient } from "@/lib/supabase/server";
import "./globals.css";

export const metadata: Metadata = {
  title: {
    default: "Omni — Build AI Agents for Any Task | Desktop App for Windows, macOS & Linux",
    template: "%s | Omni AI Agent Builder",
  },
  description:
    "Omni is a free desktop application for Windows, macOS, and Linux that lets you build AI agents with full computer control. Execute commands, read/write files, call APIs, automate workflows, and connect 21+ channels — all sandboxed, permission-gated, and running locally on your machine.",
  metadataBase: new URL(
    process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000",
  ),
  keywords: [
    "AI agent builder",
    "build AI agents",
    "AI automation software",
    "desktop AI agent",
    "AI agent app",
    "AI agent platform",
    "AI computer control",
    "AI task automation",
    "AI shell commands",
    "AI file management",
    "Windows AI agent",
    "macOS AI agent",
    "Linux AI agent",
    "local AI agent",
    "privacy AI agent",
    "sandboxed AI agent",
    "persistent AI agent",
    "multi-channel AI agent",
    "AI tools",
    "WASM extensions",
    "LLM desktop app",
    "Omni AI",
    "open source AI agent",
    "AI agent creator",
    "AI workflow automation",
    "AI process execution",
  ],
  openGraph: {
    type: "website",
    siteName: "Omni AI",
    locale: "en_US",
    images: [
      {
        url: "/og-image.png",
        width: 1200,
        height: 630,
        alt: "Omni — Build AI Agents for Any Task on Windows, macOS & Linux",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    site: "@omniappai",
    creator: "@omniappai",
  },
  robots: {
    index: true,
    follow: true,
  },
};

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const supabase = await createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  return (
    <html lang="en" className="dark">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        <link
          href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap"
          rel="stylesheet"
        />
      </head>
      <body className="min-h-screen flex flex-col font-sans">
        <Navbar user={user ? { id: user.id, email: user.email || "" } : null} />
        <main className="flex-1">{children}</main>
        <Footer />
        <Toaster theme="dark" position="bottom-right" />
        <Script
          src="https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit"
          strategy="afterInteractive"
        />
      </body>
    </html>
  );
}
