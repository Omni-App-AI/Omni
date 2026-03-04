import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { logSecurityEvent } from "@/lib/anti-bot/security-logger";

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ flagId: string }> },
) {
  const { flagId } = await params;

  // Auth + moderator check
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();

  if (!profile || !(profile as any).is_moderator) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json();
  const { status, moderator_note, action } = body as {
    status?: string;
    moderator_note?: string;
    action?: "dismiss" | "remove_content" | "warn_user" | "ban_user";
  };

  const validStatuses = ["reviewed", "actioned", "dismissed"];
  if (!status || !validStatuses.includes(status)) {
    return NextResponse.json(
      { error: `status must be one of: ${validStatuses.join(", ")}` },
      { status: 400 }
    );
  }

  // Get the flag
  const { data: flag } = await service
    .from("content_flags")
    .select("*")
    .eq("id", flagId)
    .single();

  if (!flag) {
    return NextResponse.json({ error: "Flag not found" }, { status: 404 });
  }

  // Update the flag
  const { error: updateError } = await (service
    .from("content_flags") as any)
    .update({
      status,
      moderator_id: user.id,
      moderator_note: moderator_note || null,
      resolved_at: new Date().toISOString(),
    })
    .eq("id", flagId);

  if (updateError) {
    return NextResponse.json({ error: updateError.message }, { status: 500 });
  }

  // Handle actions
  if (action === "remove_content") {
    const flagData = flag as any;
    if (flagData.content_type === "post") {
      await service.from("forum_posts").delete().eq("id", flagData.content_id);
    } else if (flagData.content_type === "reply") {
      await service.from("forum_replies").delete().eq("id", flagData.content_id);
    } else if (flagData.content_type === "review") {
      await service.from("reviews").delete().eq("id", flagData.content_id);
    } else if (flagData.content_type === "extension") {
      await (service.from("extensions") as any)
        .update({
          published: false,
          moderation_status: "taken_down",
          moderation_note: moderator_note || "Taken down via content flag",
          moderated_by: user.id,
          moderated_at: new Date().toISOString(),
        })
        .eq("id", flagData.content_id);
    }

    await logSecurityEvent({
      eventType: "content_removed",
      actorId: user.id,
      metadata: {
        flag_id: flagId,
        content_type: flagData.content_type,
        content_id: flagData.content_id,
        action,
      },
    });
  }

  return NextResponse.json({ success: true, flag_id: flagId, status });
}
