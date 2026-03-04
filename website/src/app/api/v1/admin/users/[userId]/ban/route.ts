import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { logSecurityEvent } from "@/lib/anti-bot/security-logger";

async function checkModerator(userId: string): Promise<boolean> {
  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", userId)
    .single();
  return !!(profile as any)?.is_moderator;
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ userId: string }> },
) {
  const { userId: targetUserId } = await params;

  // Auth + moderator check
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!(await checkModerator(user.id))) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  if (targetUserId === user.id) {
    return NextResponse.json({ error: "Cannot ban yourself" }, { status: 400 });
  }

  const body = await request.json();
  const { ban_type, reason, duration_hours } = body as {
    ban_type?: string;
    reason?: string;
    duration_hours?: number;
  };

  if (!ban_type || !reason) {
    return NextResponse.json(
      { error: "ban_type and reason are required" },
      { status: 400 }
    );
  }

  const validTypes = ["temporary", "permanent", "shadow"];
  if (!validTypes.includes(ban_type)) {
    return NextResponse.json(
      { error: `ban_type must be one of: ${validTypes.join(", ")}` },
      { status: 400 }
    );
  }

  if (ban_type === "temporary" && !duration_hours) {
    return NextResponse.json(
      { error: "duration_hours is required for temporary bans" },
      { status: 400 }
    );
  }

  const service = createServiceClient();

  // Calculate expiration for temporary bans
  const expiresAt = ban_type === "temporary" && duration_hours
    ? new Date(Date.now() + duration_hours * 60 * 60 * 1000).toISOString()
    : null;

  // Create ban record
  const { error: banError } = await (service.from("user_bans") as any).insert({
    user_id: targetUserId,
    ban_type,
    reason,
    banned_by: user.id,
    expires_at: expiresAt,
  });

  if (banError) {
    return NextResponse.json({ error: banError.message }, { status: 500 });
  }

  // Update profile
  await (service
    .from("profiles") as any)
    .update({
      is_banned: true,
      ban_reason: reason,
    })
    .eq("id", targetUserId);

  await logSecurityEvent({
    eventType: "account_banned",
    actorId: user.id,
    metadata: {
      target_user_id: targetUserId,
      ban_type,
      reason,
      duration_hours,
      expires_at: expiresAt,
    },
  });

  return NextResponse.json({
    success: true,
    ban_type,
    expires_at: expiresAt,
  }, { status: 201 });
}

export async function DELETE(
  _request: NextRequest,
  { params }: { params: Promise<{ userId: string }> },
) {
  const { userId: targetUserId } = await params;

  // Auth + moderator check
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!(await checkModerator(user.id))) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const service = createServiceClient();

  // Revoke all active bans
  await (service
    .from("user_bans") as any)
    .update({
      revoked_at: new Date().toISOString(),
      revoked_by: user.id,
    })
    .eq("user_id", targetUserId)
    .is("revoked_at", null);

  // Clear ban from profile
  await (service
    .from("profiles") as any)
    .update({
      is_banned: false,
      ban_reason: null,
    })
    .eq("id", targetUserId);

  await logSecurityEvent({
    eventType: "account_unbanned",
    actorId: user.id,
    metadata: { target_user_id: targetUserId },
  });

  return NextResponse.json({ success: true });
}
