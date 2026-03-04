import { NextResponse, type NextRequest } from "next/server";
import Stripe from "stripe";
import { createServiceClient } from "@/lib/supabase/server";

function getStripe() {
  return new Stripe(process.env.STRIPE_SECRET_KEY!);
}

function getWebhookSecret() {
  return process.env.STRIPE_WEBHOOK_SECRET!;
}

async function grantDonorBadge(
  supabase: ReturnType<typeof createServiceClient>,
  userId: string,
) {
  // Upsert badge -- ignoreDuplicates maps to ON CONFLICT DO NOTHING,
  // so repeat donations skip cleanly instead of hitting a constraint error.
  // Type assertion needed because user_badges has Update: never which
  // causes the Supabase client to infer insert params as never.
  await (supabase.from("user_badges") as any).upsert(
    { user_id: userId, badge_id: "donor" },
    { onConflict: "user_id,badge_id", ignoreDuplicates: true },
  );
}

export async function POST(request: NextRequest) {
  const body = await request.text();
  const sig = request.headers.get("stripe-signature");

  if (!sig) {
    return NextResponse.json({ error: "Missing signature" }, { status: 400 });
  }

  const stripe = getStripe();

  let event: Stripe.Event;
  try {
    event = stripe.webhooks.constructEvent(body, sig, getWebhookSecret());
  } catch (err) {
    const message = err instanceof Error ? err.message : "Signature verification failed";
    return NextResponse.json({ error: message }, { status: 400 });
  }

  const supabase = createServiceClient();

  if (event.type === "checkout.session.completed") {
    const session = event.data.object as Stripe.Checkout.Session;

    // Skip subscription checkouts -- they'll be handled by invoice.payment_succeeded
    if (session.mode === "subscription") {
      // Still grant badge for first subscription
      const userId = session.metadata?.user_id;
      if (userId) {
        await grantDonorBadge(supabase, userId);
      }
      return NextResponse.json({ received: true });
    }

    // One-time payment
    const result = await recordDonation(supabase, {
      stripeId: session.id,
      amountCents: session.amount_total ?? 0,
      currency: session.currency ?? "usd",
      recurring: false,
      userId: session.metadata?.user_id || null,
      donorName: session.metadata?.donor_name || null,
      showOnList: session.metadata?.show_on_list === "true",
    });

    if (result.error) {
      return NextResponse.json({ error: result.error }, { status: 500 });
    }
  }

  if (event.type === "invoice.payment_succeeded") {
    const invoice = event.data.object as Stripe.Invoice;

    // Only handle subscription invoices
    const isSubscription =
      invoice.billing_reason === "subscription_create" ||
      invoice.billing_reason === "subscription_cycle" ||
      invoice.billing_reason === "subscription_update";

    if (!isSubscription) {
      return NextResponse.json({ received: true });
    }

    const metadata = invoice.parent?.subscription_details?.metadata ?? {};

    const result = await recordDonation(supabase, {
      stripeId: invoice.id,
      amountCents: invoice.amount_paid,
      currency: invoice.currency ?? "usd",
      recurring: true,
      userId: metadata.user_id || null,
      donorName: metadata.donor_name || null,
      showOnList: metadata.show_on_list === "true",
    });

    if (result.error) {
      return NextResponse.json({ error: result.error }, { status: 500 });
    }
  }

  return NextResponse.json({ received: true });
}

async function recordDonation(
  supabase: ReturnType<typeof createServiceClient>,
  params: {
    stripeId: string;
    amountCents: number;
    currency: string;
    recurring: boolean;
    userId: string | null;
    donorName: string | null;
    showOnList: boolean;
  },
): Promise<{ error?: string }> {
  if (params.amountCents <= 0) return {};

  // Insert donation (idempotent via unique stripe_session_id -- duplicates are ignored)
  // Type assertion needed because donations Update type is restricted,
  // which causes Supabase upsert to infer params as never
  const { error } = await (supabase.from("donations") as any).upsert(
    {
      stripe_session_id: params.stripeId,
      amount_cents: params.amountCents,
      currency: params.currency,
      recurring: params.recurring,
      user_id: params.userId,
      donor_name: params.donorName,
      show_on_list: params.showOnList,
    },
    { onConflict: "stripe_session_id" },
  );

  if (error) {
    console.error("Failed to record donation:", error);
    return { error: error.message };
  }

  // Grant donor badge if user is authenticated
  if (params.userId) {
    await grantDonorBadge(supabase, params.userId);
  }

  return {};
}
