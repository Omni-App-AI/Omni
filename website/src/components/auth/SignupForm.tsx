"use client";

import { useState, useRef } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { OAuthButtons } from "./OAuthButtons";
import { Turnstile } from "@/components/ui/Turnstile";
import { HoneypotFields } from "@/components/ui/HoneypotFields";

export function SignupForm() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [username, setUsername] = useState("");
  const [fullName, setFullName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null);
  const formRef = useRef<HTMLFormElement>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    if (!/^[a-z0-9_-]{3,39}$/.test(username)) {
      setError("Username must be 3-39 characters: lowercase letters, numbers, hyphens, underscores.");
      setLoading(false);
      return;
    }

    if (!turnstileToken) {
      setError("Please complete the verification challenge.");
      setLoading(false);
      return;
    }

    // Collect honeypot fields from the form
    const formData = new FormData(formRef.current!);

    try {
      const response = await fetch("/api/v1/auth/signup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password,
          username,
          full_name: fullName,
          turnstile_token: turnstileToken,
          hp_website: formData.get("hp_website") || "",
          hp_timestamp: formData.get("hp_timestamp") || "",
          hp_token: formData.get("hp_token") || "",
        }),
      });

      const data = await response.json();

      if (!response.ok) {
        setError(data.error || "Signup failed. Please try again.");
        setLoading(false);
        return;
      }

      setSuccess(true);
    } catch {
      setError("Network error. Please try again.");
    }

    setLoading(false);
  };

  if (success) {
    return (
      <div className="w-full max-w-sm mx-auto text-center space-y-4">
        <h1 className="text-2xl font-bold">Check your email</h1>
        <p className="text-muted-foreground">
          We sent a confirmation link to <strong>{email}</strong>. Click the link to activate your account.
        </p>
        <Link href="/login">
          <Button variant="outline" className="mt-4">Back to login</Button>
        </Link>
      </div>
    );
  }

  return (
    <div className="w-full max-w-sm mx-auto space-y-6">
      <div className="text-center">
        <h1 className="text-2xl font-bold">Create your account</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Join the Omni extension ecosystem
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
          <label htmlFor="fullName" className="text-sm font-medium">
            Full name
          </label>
          <Input
            id="fullName"
            type="text"
            placeholder="Jane Doe"
            value={fullName}
            onChange={(e) => setFullName(e.target.value)}
            required
            className="mt-1"
          />
        </div>
        <div>
          <label htmlFor="username" className="text-sm font-medium">
            Username
          </label>
          <Input
            id="username"
            type="text"
            placeholder="janedoe"
            value={username}
            onChange={(e) => setUsername(e.target.value.toLowerCase())}
            required
            pattern="^[a-z0-9_-]{3,39}$"
            className="mt-1"
          />
          <p className="mt-1 text-xs text-muted-foreground">
            3-39 characters. Letters, numbers, hyphens, underscores.
          </p>
        </div>
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
            placeholder="At least 8 characters"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={8}
            className="mt-1"
          />
        </div>

        <Turnstile
          mode="managed"
          onVerify={setTurnstileToken}
          onExpire={() => setTurnstileToken(null)}
          className="flex justify-center"
        />

        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}

        <Button type="submit" className="w-full" disabled={loading || !turnstileToken}>
          {loading ? "Creating account..." : "Create account"}
        </Button>
      </form>

      <p className="text-center text-sm text-muted-foreground">
        Already have an account?{" "}
        <Link href="/login" className="text-primary hover:underline">
          Sign in
        </Link>
      </p>
    </div>
  );
}
