"use client";

import { useEffect, useRef, useCallback, useState } from "react";

interface TurnstileProps {
  siteKey?: string;
  mode?: "managed" | "invisible";
  onVerify: (token: string) => void;
  onError?: () => void;
  onExpire?: () => void;
  className?: string;
}

declare global {
  interface Window {
    turnstile?: {
      render: (
        container: string | HTMLElement,
        options: Record<string, unknown>
      ) => string;
      reset: (widgetId: string) => void;
      getResponse: (widgetId: string) => string | undefined;
      remove: (widgetId: string) => void;
    };
    onTurnstileLoad?: () => void;
  }
}

const TURNSTILE_DISABLED = process.env.NEXT_PUBLIC_TURNSTILE_DISABLED === "true";

export function Turnstile({
  siteKey,
  mode = "managed",
  onVerify,
  onError,
  onExpire,
  className,
}: TurnstileProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const widgetIdRef = useRef<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  const effectiveKey = siteKey || process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY || "";

  // Store callbacks in refs to avoid re-rendering the widget when they change.
  // This is critical -- inline arrows like `onExpire={() => setToken(null)}`
  // create new references every render, which would otherwise cause an
  // infinite remove-and-re-render loop via the useCallback/useEffect chain.
  const onVerifyRef = useRef(onVerify);
  const onErrorRef = useRef(onError);
  const onExpireRef = useRef(onExpire);

  useEffect(() => {
    onVerifyRef.current = onVerify;
    onErrorRef.current = onError;
    onExpireRef.current = onExpire;
  });

  // Dev bypass: auto-fire onVerify with a dummy token and render nothing
  useEffect(() => {
    if (TURNSTILE_DISABLED) {
      onVerifyRef.current("turnstile-disabled-dev");
    }
  }, []);

  const renderWidget = useCallback(() => {
    if (TURNSTILE_DISABLED) return;
    if (!window.turnstile || !containerRef.current || widgetIdRef.current) return;
    if (!effectiveKey) {
      console.warn("[Turnstile] No site key configured");
      return;
    }

    widgetIdRef.current = window.turnstile.render(containerRef.current, {
      sitekey: effectiveKey,
      size: mode === "invisible" ? "invisible" : "normal",
      callback: (token: string) => onVerifyRef.current(token),
      "error-callback": () => onErrorRef.current?.(),
      "expired-callback": () => onExpireRef.current?.(),
      theme: "auto",
      appearance: mode === "invisible" ? "interaction-only" : "always",
    });
  }, [effectiveKey, mode]); // Only re-render widget if key or mode changes

  // Load the Turnstile script
  useEffect(() => {
    if (TURNSTILE_DISABLED) return;

    // Already available
    if (window.turnstile) {
      setLoaded(true);
      return;
    }

    // Poll for window.turnstile -- handles the case where the <Script> in
    // layout.tsx loaded the API but the `load` event fired before this
    // effect mounted (race condition with strategy="afterInteractive").
    const interval = setInterval(() => {
      if (window.turnstile) {
        setLoaded(true);
        clearInterval(interval);
      }
    }, 200);

    // Also check if a script tag already exists (from layout.tsx or a
    // previous mount) and listen for its load event as a backup.
    const existing = document.querySelector(
      'script[src*="challenges.cloudflare.com/turnstile"]'
    );

    if (existing) {
      const onLoad = () => {
        if (window.turnstile) setLoaded(true);
        clearInterval(interval);
      };
      existing.addEventListener("load", onLoad);
      return () => {
        clearInterval(interval);
        existing.removeEventListener("load", onLoad);
      };
    }

    // No script tag found -- load it ourselves
    const script = document.createElement("script");
    script.src =
      "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";
    script.async = true;
    script.onload = () => {
      setLoaded(true);
      clearInterval(interval);
    };
    document.head.appendChild(script);

    return () => {
      clearInterval(interval);
    };
  }, []);

  // Render / clean up the widget
  useEffect(() => {
    if (TURNSTILE_DISABLED) return;
    if (loaded) {
      renderWidget();
    }

    return () => {
      if (widgetIdRef.current && window.turnstile) {
        window.turnstile.remove(widgetIdRef.current);
        widgetIdRef.current = null;
      }
    };
  }, [loaded, renderWidget]);

  // Conditional render AFTER all hooks (Rules of Hooks compliance)
  if (TURNSTILE_DISABLED) {
    return null;
  }

  return <div ref={containerRef} className={className} />;
}

/**
 * Hook for programmatic Turnstile usage.
 * Returns the current token and a reset function.
 */
export function useTurnstile() {
  const [token, setToken] = useState<string | null>(null);

  const reset = useCallback(() => {
    setToken(null);
  }, []);

  return { token, setToken, reset };
}
