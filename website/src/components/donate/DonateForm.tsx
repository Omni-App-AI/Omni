"use client";

import { useState, useEffect } from "react";
import { Heart, Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { createClient } from "@/lib/supabase/client";

const PRESET_AMOUNTS = [500, 1000, 2500, 5000, 10000]; // cents

export function DonateForm() {
  const [selectedAmount, setSelectedAmount] = useState(1000); // $10 default
  const [customAmount, setCustomAmount] = useState("");
  const [isCustom, setIsCustom] = useState(false);
  const [recurring, setRecurring] = useState(false);
  const [showOnList, setShowOnList] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [user, setUser] = useState<{ id: string; display_name?: string } | null>(null);

  useEffect(() => {
    const supabase = createClient();
    supabase.auth.getUser().then(async ({ data: { user: authUser } }) => {
      if (authUser) {
        const { data } = await supabase
          .from("profiles")
          .select("display_name")
          .eq("id", authUser.id)
          .single();

        const profile = data as { display_name: string } | null;

        setUser({
          id: authUser.id,
          display_name: profile?.display_name || undefined,
        });
      }
    });
  }, []);

  const amountCents = isCustom
    ? Math.round(parseFloat(customAmount || "0") * 100)
    : selectedAmount;

  const amountDisplay = (amountCents / 100).toFixed(amountCents % 100 === 0 ? 0 : 2);

  const handlePresetClick = (amount: number) => {
    setSelectedAmount(amount);
    setIsCustom(false);
    setCustomAmount("");
    setError(null);
  };

  const handleCustomFocus = () => {
    setIsCustom(true);
    setError(null);
  };

  const handleSubmit = async () => {
    if (!Number.isFinite(amountCents) || amountCents < 100) {
      setError("Minimum donation is $1.00");
      return;
    }

    if (amountCents > 99999900) {
      setError("Maximum donation is $999,999.00");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const supabase = createClient();
      const { data, error: fnError } = await supabase.functions.invoke(
        "create-checkout",
        {
          body: {
            amount_cents: amountCents,
            show_on_list: showOnList,
            recurring,
            return_url: `${window.location.origin}/donate?thanks=1`,
            user_id: user?.id || null,
            donor_name: showOnList ? user?.display_name || null : null,
          },
        },
      );

      if (fnError) {
        setError(fnError.message || "Failed to create checkout session");
        setLoading(false);
        return;
      }

      if (data?.url) {
        window.location.href = data.url;
      } else {
        setError("No checkout URL returned");
        setLoading(false);
      }
    } catch {
      setError("Something went wrong. Please try again.");
      setLoading(false);
    }
  };

  return (
    <div className="border-gradient rounded-xl glow-sm">
      <div className="rounded-xl bg-card p-6 sm:p-8 space-y-6">
        {/* Card header */}
        <div>
          <h2 className="text-lg font-semibold tracking-tight">Make a donation</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Choose an amount and frequency below.
          </p>
        </div>

        <div className="h-px bg-border/50" />

        {/* Frequency toggle */}
        <div>
          <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground/60 mb-3">
            Frequency
          </p>
          <div className="grid grid-cols-2 gap-2">
            <button
              type="button"
              onClick={() => setRecurring(false)}
              className={`relative h-11 rounded-lg border text-sm font-medium transition-all ${
                !recurring
                  ? "border-primary bg-primary/10 text-primary shadow-[0_0_12px_-4px_rgba(124,107,245,0.3)]"
                  : "border-border/50 text-muted-foreground hover:border-border hover:text-foreground"
              }`}
            >
              One-time
            </button>
            <button
              type="button"
              onClick={() => setRecurring(true)}
              className={`relative h-11 rounded-lg border text-sm font-medium transition-all ${
                recurring
                  ? "border-primary bg-primary/10 text-primary shadow-[0_0_12px_-4px_rgba(124,107,245,0.3)]"
                  : "border-border/50 text-muted-foreground hover:border-border hover:text-foreground"
              }`}
            >
              Monthly
            </button>
          </div>
        </div>

        {/* Amount selection */}
        <div>
          <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground/60 mb-3">
            Amount
          </p>
          <div className="grid grid-cols-5 gap-2 mb-3">
            {PRESET_AMOUNTS.map((amount) => (
              <button
                key={amount}
                type="button"
                onClick={() => handlePresetClick(amount)}
                className={`h-11 rounded-lg border text-sm font-medium transition-all ${
                  !isCustom && selectedAmount === amount
                    ? "border-primary bg-primary/10 text-primary shadow-[0_0_12px_-4px_rgba(124,107,245,0.3)]"
                    : "border-border/50 text-muted-foreground hover:border-border hover:text-foreground"
                }`}
              >
                ${amount / 100}
              </button>
            ))}
          </div>

          <div className="relative">
            <span className="absolute left-3.5 top-1/2 -translate-y-1/2 text-muted-foreground/60 text-sm font-medium">
              $
            </span>
            <input
              type="number"
              min="1"
              step="1"
              placeholder="Custom amount"
              value={customAmount}
              onFocus={handleCustomFocus}
              onChange={(e) => {
                setCustomAmount(e.target.value);
                setIsCustom(true);
                setError(null);
              }}
              className={`w-full h-11 pl-7 pr-4 rounded-lg border text-sm bg-secondary/40 outline-none transition-all placeholder:text-muted-foreground/40 ${
                isCustom
                  ? "border-primary text-foreground shadow-[0_0_12px_-4px_rgba(124,107,245,0.2)]"
                  : "border-border/50 text-muted-foreground hover:border-border"
              } focus:border-primary focus:shadow-[0_0_12px_-4px_rgba(124,107,245,0.2)]`}
            />
          </div>
        </div>

        {/* Show on list */}
        <label className="flex items-start gap-3 cursor-pointer group p-3 -mx-3 rounded-lg hover:bg-secondary/40 transition-colors">
          <div className="relative mt-0.5">
            <input
              type="checkbox"
              checked={showOnList}
              onChange={(e) => setShowOnList(e.target.checked)}
              className="peer sr-only"
            />
            <div className="h-4 w-4 rounded border border-border/80 bg-secondary/60 transition-all peer-checked:border-primary peer-checked:bg-primary peer-focus-visible:ring-2 peer-focus-visible:ring-ring peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-card" />
            <svg
              className="absolute inset-0 h-4 w-4 text-primary-foreground opacity-0 peer-checked:opacity-100 transition-opacity pointer-events-none"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M4 8.5L6.5 11L12 5.5" />
            </svg>
          </div>
          <div>
            <span className="text-sm font-medium group-hover:text-foreground transition-colors">
              Show me on the public supporters list
            </span>
            <p className="text-xs text-muted-foreground/60 mt-0.5">
              Your display name and donation amount will be visible.
            </p>
          </div>
        </label>

        <div className="h-px bg-border/50" />

        {/* Error */}
        {error && (
          <div className="rounded-lg border border-destructive/20 bg-destructive/5 px-4 py-3">
            <p className="text-sm text-destructive">{error}</p>
          </div>
        )}

        {/* Submit */}
        <Button
          size="xl"
          className="w-full gap-2"
          onClick={handleSubmit}
          disabled={loading || amountCents < 100}
        >
          {loading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Redirecting to Stripe…
            </>
          ) : (
            <>
              <Heart className="h-4 w-4" />
              Donate ${amountDisplay}
              {recurring ? "/month" : ""}
            </>
          )}
        </Button>

        {/* Login hint */}
        {!user && (
          <div className="flex items-center justify-center gap-1.5 text-xs text-muted-foreground/60">
            <Sparkles className="h-3 w-3" />
            <span>
              <a href="/login" className="text-primary hover:underline">
                Log in
              </a>{" "}
              to earn a Donor badge on your profile.
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
