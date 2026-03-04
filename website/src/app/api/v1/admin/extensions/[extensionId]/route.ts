import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { logSecurityEvent } from "@/lib/anti-bot/security-logger";

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ extensionId: string }> },
) {
  const { extensionId } = await params;

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
  const { action, note } = body as {
    action: "take_down" | "request_review" | "approve";
    note?: string;
  };

  const validActions = ["take_down", "request_review", "approve"];
  if (!action || !validActions.includes(action)) {
    return NextResponse.json(
      { error: `action must be one of: ${validActions.join(", ")}` },
      { status: 400 },
    );
  }

  // Verify extension exists
  const { data: ext } = await service
    .from("extensions")
    .select("id, name, published, moderation_status")
    .eq("id", extensionId)
    .single();

  if (!ext) {
    return NextResponse.json({ error: "Extension not found" }, { status: 404 });
  }

  // Build the update
  const now = new Date().toISOString();
  let update: Record<string, unknown>;

  switch (action) {
    case "take_down":
      update = {
        published: false,
        moderation_status: "taken_down",
        moderation_note: note || null,
        moderated_by: user.id,
        moderated_at: now,
      };
      break;
    case "request_review":
      update = {
        moderation_status: "under_review",
        moderation_note: note || null,
        moderated_by: user.id,
        moderated_at: now,
      };
      break;
    case "approve":
      update = {
        published: true,
        moderation_status: "active",
        moderation_note: null,
        moderated_by: user.id,
        moderated_at: now,
      };
      break;
  }

  const { error: updateError } = await (service
    .from("extensions") as any)
    .update(update)
    .eq("id", extensionId);

  if (updateError) {
    return NextResponse.json({ error: updateError.message }, { status: 500 });
  }

  await logSecurityEvent({
    eventType: "extension_moderated",
    actorId: user.id,
    metadata: {
      extension_id: extensionId,
      extension_name: (ext as any).name,
      action,
      note: note || null,
      previous_status: (ext as any).moderation_status,
    },
  });

  return NextResponse.json({
    success: true,
    extension_id: extensionId,
    action,
    moderation_status: update.moderation_status,
  });
}
