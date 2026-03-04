"use client";

import { useState, useRef } from "react";
import Link from "next/link";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { OAuthButtons } from "./OAuthButtons";
import { Turnstile } from "@/components/ui/Turnstile";
import { HoneypotFields } from "@/components/ui/HoneypotFields";

export function LoginForm({ redirect }: { redirect?: string }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null);
  const formRef = useRef<HTMLFormElement>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    if (!turnstileToken) {
      setError("Please wait for verification to complete.");
      setLoading(false);
      return;
    }

    // Collect honeypot fields
    const formData = new FormData(formRef.current!);

    try {
      // Verify through server-side route first (rate limit + Turnstile + honeypot)
      const verifyResponse = await fetch("/api/v1/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password,
          turnstile_token: turnstileToken,
          hp_website: formData.get("hp_website") || "",
          hp_timestamp: formData.get("hp_timestamp") || "",
          hp_token: formData.get("hp_token") || "",
        }),
      });

      const data = await verifyResponse.json();

      if (!verifyResponse.ok) {
        setError(data.error || "Login failed. Please try again.");
        setLoading(false);
        return;
      }

      // Server-side auth succeeded -- now authenticate the client-side Supabase session
      const supabase = createClient();
      const { error: clientError } = await supabase.auth.signInWithPassword({ email, password });

      if (clientError) {
        setError(clientError.message);
        setLoading(false);
        return;
      }

      // Hard redirect to ensure cookies are sent fresh with the new request.
      // router.push() + router.refresh() can conflict (refresh cancels push),
      // and soft navigation may not pick up the newly-set session cookies.
      window.location.href = redirect || "/dashboard";
    } catch {
      setError("Network error. Please try again.");
      setLoading(false);
    }
  };

  return (
    <div className="w-full max-w-sm mx-auto space-y-6">
      <div className="text-center">
        <h1 className="text-2xl font-bold">Welcome back</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Sign in to your Omni account
        </p>
      </div>

      <OAuthButtons />

      <div className="relative">
        <div className="absolute inset-0 flex items-center">
          <div className="w-full border-t border-border" />
        </div>
        <div className="relative flex justify-center text-xs uppercase">
          <span className="bg-background px-2 text-muted-foreground">or</span>
        </div>
      </div>

      <form ref={formRef} onSubmit={handleSubmit} className="space-y-4">
        <HoneypotFields />

        <div>
          <label htmlFor="email" className="text-sm font-medium">
            Email
          </label>
          <Input
            id="email"
            type="email"
            placeholder="you@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            className="mt-1"
          />
        </div>
        <div>
          <label htmlFor="password" className="text-sm font-medium">
            Password
          </label>
          <Input
            id="password"
            type="password"
            placeholder="Your password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            className="mt-1"
          />
        </div>

        {/* Invisible Turnstile — auto-verifies */}
        <Turnstile
          mode="invisible"
          onVerify={setTurnstileToken}
          onExpire={() => setTurnstileToken(null)}
        />

        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}

        <Button type="submit" className="w-full" disabled={loading}>
          {loading ? "Signing in..." : "Sign in"}
        </Button>
      </form>

      <p className="text-center text-sm text-muted-foreground">
        Don&apos;t have an account?{" "}
        <Link href="/signup" className="text-primary hover:underline">
          Sign up
        </Link>
      </p>
    </div>
  );
}
