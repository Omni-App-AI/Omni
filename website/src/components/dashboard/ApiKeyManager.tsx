"use client";

import { useState, useEffect, useCallback } from "react";
import { Key, Plus, Trash2, Copy, Check } from "lucide-react";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";

interface ApiKeyRow {
  id: string;
  name: string;
  key_prefix: string;
  last_used_at: string | null;
  revoked: boolean;
  created_at: string;
}

export function ApiKeyManager() {
  const [keys, setKeys] = useState<ApiKeyRow[]>([]);
  const [showNew, setShowNew] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  const [generatedKey, setGeneratedKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [loading, setLoading] = useState(false);

  const loadKeys = useCallback(async () => {
    const supabase = createClient();
    const { data } = await supabase
      .from("api_keys")
      .select("id, name, key_prefix, last_used_at, revoked, created_at")
      .order("created_at", { ascending: false });
    setKeys(data || []);
  }, []);

  useEffect(() => {
    loadKeys();
  }, [loadKeys]);

  const generateKey = async () => {
    if (!newKeyName) return;
    setLoading(true);

    const supabase = createClient();
    const { data: { user } } = await supabase.auth.getUser();
    if (!user) return;

    // Generate random key
    const randomBytes = crypto.getRandomValues(new Uint8Array(32));
    const keyHex = Array.from(randomBytes).map((b) => b.toString(16).padStart(2, "0")).join("");
    const fullKey = `omni_pk_${keyHex}`;
    const prefix = fullKey.substring(0, 16);

    // Hash the key (simple SHA-256 for storage)
    const encoder = new TextEncoder();
    const hashBuffer = await crypto.subtle.digest("SHA-256", encoder.encode(fullKey));
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const keyHash = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

    const insertData: any = {
      user_id: user.id,
      name: newKeyName,
      key_hash: keyHash,
      key_prefix: prefix,
    };
    await supabase.from("api_keys").insert(insertData);

    setGeneratedKey(fullKey);
    setShowNew(false);
    setNewKeyName("");
    setLoading(false);
    await loadKeys();
  };

  const revokeKey = async (keyId: string) => {
    const supabase = createClient();
    // @ts-expect-error -- Supabase type inference limitation with manual Database type
    await supabase.from("api_keys").update({ revoked: true }).eq("id", keyId);
    await loadKeys();
  };

  const copyKey = () => {
    if (generatedKey) {
      navigator.clipboard.writeText(generatedKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="space-y-6">
      {/* Generated key alert */}
      {generatedKey && (
        <div className="border border-success/30 bg-success/5 rounded-lg p-5">
          <p className="text-sm font-medium text-success mb-3">
            API key created! Copy it now — you won&apos;t see it again.
          </p>
          <div className="flex items-center gap-2">
            <code className="flex-1 bg-background rounded-md px-3 py-2 text-[13px] font-mono break-all border border-border/50">
              {generatedKey}
            </code>
            <Button variant="outline" size="icon" onClick={copyKey}>
              {copied ? <Check className="h-4 w-4 text-success" /> : <Copy className="h-4 w-4" />}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground mt-3 font-mono">
            omni ext publish --api-key YOUR_KEY
          </p>
        </div>
      )}

      {/* Create new key */}
      {showNew ? (
        <div className="border border-border/50 rounded-lg p-5">
          <label className="text-[13px] font-medium mb-2 block">Key name</label>
          <div className="flex items-center gap-3">
            <Input
              placeholder="My Laptop, CI Pipeline..."
              value={newKeyName}
              onChange={(e) => setNewKeyName(e.target.value)}
              className="flex-1"
            />
            <Button onClick={generateKey} disabled={!newKeyName || loading} size="sm">
              Generate
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setShowNew(false)}>
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <Button onClick={() => setShowNew(true)} className="gap-2" size="sm">
          <Plus className="h-4 w-4" />
          Create API Key
        </Button>
      )}

      {/* Key list */}
      <div>
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          Your API Keys
        </h2>
        {keys.length === 0 ? (
          <div className="border border-dashed border-border/50 rounded-lg p-8 text-center">
            <Key className="h-8 w-8 text-muted-foreground/40 mx-auto mb-3" />
            <p className="text-sm text-muted-foreground">
              No API keys yet. Create one to publish extensions via the CLI.
            </p>
          </div>
        ) : (
          <div className="border border-border/50 rounded-lg divide-y divide-border/50">
            {keys.map((key) => (
              <div
                key={key.id}
                className="flex items-center justify-between px-5 py-4"
              >
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">{key.name}</span>
                    {key.revoked && <Badge variant="destructive">Revoked</Badge>}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1 flex items-center gap-2">
                    <code className="font-mono">{key.key_prefix}...</code>
                    <span className="text-muted-foreground/40">·</span>
                    <span>Created {new Date(key.created_at).toLocaleDateString()}</span>
                    {key.last_used_at && (
                      <>
                        <span className="text-muted-foreground/40">·</span>
                        <span>Last used {new Date(key.last_used_at).toLocaleDateString()}</span>
                      </>
                    )}
                  </div>
                </div>
                {!key.revoked && (
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => revokeKey(key.id)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
