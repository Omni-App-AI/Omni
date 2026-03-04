import type { Metadata } from "next";
import { SignupForm } from "@/components/auth/SignupForm";

export const metadata: Metadata = {
  title: "Sign up",
  description:
    "Create a free Omni Marketplace account to publish sandboxed WASM extensions, review AI agent tools, join the developer community, and manage your extension portfolio.",
  robots: { index: false, follow: false },
};

export default function SignupPage() {
  return (
    <div className="flex min-h-[calc(100vh-8rem)] items-center justify-center px-4 py-12">
      <SignupForm />
    </div>
  );
}
