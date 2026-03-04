import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import Stripe from "https://esm.sh/stripe@14.14.0?target=deno";

const stripe = new Stripe(Deno.env.get("STRIPE_SECRET_KEY")!, {
  apiVersion: "2024-04-10",
  httpClient: Stripe.createFetchHttpClient(),
});

const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
};

serve(async (req: Request) => {
  // Handle CORS preflight
  if (req.method === "OPTIONS") {
    return new Response("ok", { headers: corsHeaders });
  }

  try {
    const {
      amount_cents,
      show_on_list = false,
      recurring = false,
      return_url,
      user_id,
      donor_name,
    } = await req.json();

    // Validate
    if (!amount_cents || amount_cents < 100) {
      return new Response(
        JSON.stringify({ error: "Minimum donation is $1.00" }),
        { status: 400, headers: { ...corsHeaders, "Content-Type": "application/json" } },
      );
    }

    if (!return_url) {
      return new Response(
        JSON.stringify({ error: "return_url is required" }),
        { status: 400, headers: { ...corsHeaders, "Content-Type": "application/json" } },
      );
    }

    const metadata: Record<string, string> = {
      show_on_list: String(show_on_list),
    };
    if (user_id) metadata.user_id = user_id;
    if (donor_name) metadata.donor_name = donor_name;

    const sessionParams: Stripe.Checkout.SessionCreateParams = {
      success_url: return_url,
      cancel_url: return_url.replace("?thanks=1", ""),
      metadata,
      line_items: [
        {
          price_data: {
            currency: "usd",
            product_data: {
              name: recurring ? "Monthly Donation to Omni" : "Donation to Omni",
              description: "Thank you for supporting open-source AI development",
            },
            unit_amount: amount_cents,
            ...(recurring ? { recurring: { interval: "month" as const } } : {}),
          },
          quantity: 1,
        },
      ],
      mode: recurring ? "subscription" : "payment",
    };

    // For subscriptions, attach metadata to the subscription as well
    if (recurring) {
      sessionParams.subscription_data = { metadata };
    }

    const session = await stripe.checkout.sessions.create(sessionParams);

    return new Response(
      JSON.stringify({ url: session.url }),
      { status: 200, headers: { ...corsHeaders, "Content-Type": "application/json" } },
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return new Response(
      JSON.stringify({ error: message }),
      { status: 500, headers: { ...corsHeaders, "Content-Type": "application/json" } },
    );
  }
});
