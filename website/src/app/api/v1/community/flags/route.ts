import { NextResponse } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { withProtection } from "@/lib/anti-bot/with-protection";

export const POST = withProtection(
  {
    rateLimit: "flag_create",
    requireAuth: true,
  },
  async (_request, { user, body }) => {
    const { content_type, content_id, reason, details } = body as {
      content_type?: string;
      content_id?: string;
      reason?: string;
      details?: string;
    };

    if (!content_type || !content_id || !reason) {
      return NextResponse.json(
        { error: "content_type, content_id, and reason are required" },
        { status: 400 }
      );
    }

    const validTypes = ["post", "reply", "review", "extension"];
    if (!validTypes.includes(content_type)) {
      return NextResponse.json(
        { error: `content_type must be one of: ${validTypes.join(", ")}` },
        { status: 400 }
      );
    }

    const validReasons = ["spam", "harassment", "misinformation", "off_topic", "malicious", "other"];
    if (!validReasons.includes(reason)) {
      return NextResponse.json(
        { error: `reason must be one of: ${validReasons.join(", ")}` },
        { status: 400 }
      );
    }

    // Check for duplicate flag from same user on same content
    const supabase = createServiceClient();
    const { data: existing } = await supabase
      .from("content_flags")
      .select("id")
      .eq("reporter_id", user.id)
      .eq("content_type", content_type)
      .eq("content_id", content_id)
      .eq("status", "pending")
      .limit(1);

    if (existing && existing.length > 0) {
      return NextResponse.json(
        { error: "You have already flagged this content" },
        { status: 409 }
      );
    }

    const { data: flag, error } = await supabase
      .from("content_flags")
      .insert({
        content_type,
        content_id,
        reporter_id: user.id,
        reason,
        details: details || null,
        status: "pending",
      } as any)
      .select("id, content_type, content_id, reason, status, created_at")
      .single();

    if (error) {
      return NextResponse.json({ error: error.message }, { status: 500 });
    }

    return NextResponse.json({ flag }, { status: 201 });
  }
);
