"use client";

import { useEffect, useState } from "react";

/**
 * Honeypot fields component.
 * Renders three hidden anti-bot fields:
 * 1. A hidden "website" text input (bots auto-fill, humans don't see)
 * 2. An obfuscated timestamp (checks submission speed)
 * 3. A JS-generated token (proves browser JS execution)
 *
 * Include this in any form that needs bot protection.
 * The server validates via validateHoneypot() in lib/anti-bot/honeypot.ts
 */
export function HoneypotFields() {
  const [timestamp, setTimestamp] = useState("");
  const [jsToken, setJsToken] = useState("");

  useEffect(() => {
    // Generate timestamp on mount
    setTimestamp(btoa(Date.now().toString(36)));

    // Generate JS proof token
    const payload = Date.now().toString(36) + ":omni-hp-2026";
    setJsToken(btoa(payload));
  }, []);

  return (
    <>
      {/* Hidden field — bots auto-fill, humans never see */}
      <div
        aria-hidden="true"
        style={{
          position: "absolute",
          left: "-9999px",
          top: "-9999px",
          width: "1px",
          height: "1px",
          overflow: "hidden",
        }}
      >
        <label htmlFor="hp_website">Website</label>
        <input
          type="text"
          id="hp_website"
          name="hp_website"
          tabIndex={-1}
          autoComplete="off"
          defaultValue=""
        />
      </div>

      {/* Obfuscated render timestamp */}
      <input type="hidden" name="hp_timestamp" value={timestamp} />

      {/* JS execution proof token */}
      <input type="hidden" name="hp_token" value={jsToken} />
    </>
  );
}

/**
 * Extracts honeypot field values from a form body object.
 * Useful when reading form state before submitting via fetch().
 */
export function getHoneypotValues(form: HTMLFormElement): {
  hp_website: string;
  hp_timestamp: string;
  hp_token: string;
} {
  const data = new FormData(form);
  return {
    hp_website: (data.get("hp_website") as string) || "",
    hp_timestamp: (data.get("hp_timestamp") as string) || "",
    hp_token: (data.get("hp_token") as string) || "",
  };
}
