import type { Metadata } from "next";
import { LoginForm } from "@/components/auth/LoginForm";

export const metadata: Metadata = {
  title: "Log in",
  description:
    "Log in to Omni Marketplace to manage AI agent extensions, publish WASM tools, leave reviews, participate in community discussions, and track your extension downloads.",
  robots: { index: false, follow: false },
};

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ redirect?: string }>;
}) {
  const params = await searchParams;

  return (
    <div className="flex min-h-[calc(100vh-8rem)] items-center justify-center px-4 py-12">
      <LoginForm redirect={params.redirect} />
    </div>
  );
}
