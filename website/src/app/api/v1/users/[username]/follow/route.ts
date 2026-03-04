import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { withProtection } from "@/lib/anti-bot/with-protection";

export const POST = withProtection(
  {
    rateLimit: "follow_action",
    requireAuth: true,
  },
  async (_request, { user }, routeContext) => {
    const { username } = await routeContext!.params;
    const supabase = await createClient();

    // Look up the target user
    const service = createServiceClient();
    const { data: target } = await service
      .from("profiles")
      .select("id")
      .eq("username", username)
      .single();

    if (!target) {
      return NextResponse.json({ error: "User not found" }, { status: 404 });
    }

    const targetId = (target as any).id as string;

    if (targetId === user.id) {
      return NextResponse.json({ error: "Cannot follow yourself" }, { status: 400 });
    }

    // Check if already following
    const { data: existing } = await supabase
      .from("user_followers")
      .select("follower_id")
      .eq("follower_id", user.id)
      .eq("following_id", targetId);

    if (existing && existing.length > 0) {
      return NextResponse.json({ error: "Already following" }, { status: 409 });
    }

    const { error } = await supabase.from("user_followers").insert({
      follower_id: user.id,
      following_id: targetId,
    } as any);

    if (error) {
      return NextResponse.json({ error: error.message }, { status: 500 });
    }

    // Update follower counts
    const { data: targetProfile } = await service.from("profiles").select("follower_count").eq("id", targetId).single();
    const { data: myProfile } = await service.from("profiles").select("following_count").eq("id", user.id).single();

    if (targetProfile) {
      await service
        .from("profiles")
        // @ts-expect-error -- Supabase generic chain infers Update as never
        .update({ follower_count: ((targetProfile as any).follower_count || 0) + 1 })
        .eq("id", targetId);
    }
    if (myProfile) {
      await service
        .from("profiles")
        // @ts-expect-error -- Supabase generic chain infers Update as never
        .update({ following_count: ((myProfile as any).following_count || 0) + 1 })
        .eq("id", user.id);
    }

    return NextResponse.json({ success: true, following: true }, { status: 201 });
  }
);

export async function DELETE(
  _request: Request,
  { params }: { params: Promise<{ username: string }> },
) {
  const { username } = await params;
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const service = createServiceClient();
  const { data: target } = await service
    .from("profiles")
    .select("id")
    .eq("username", username)
    .single();

  if (!target) {
    return NextResponse.json({ error: "User not found" }, { status: 404 });
  }

  const targetId = (target as any).id as string;

  const { error } = await supabase
    .from("user_followers")
    .delete()
    .eq("follower_id", user.id)
    .eq("following_id", targetId);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  // Update follower counts
  const { data: targetProfile } = await service.from("profiles").select("follower_count").eq("id", targetId).single();
  const { data: myProfile } = await service.from("profiles").select("following_count").eq("id", user.id).single();

  if (targetProfile) {
    await service
      .from("profiles")
      // @ts-expect-error -- Supabase generic chain infers Update as never
      .update({ follower_count: Math.max(0, ((targetProfile as any).follower_count || 0) - 1) })
      .eq("id", targetId);
  }
  if (myProfile) {
    await service
      .from("profiles")
      // @ts-expect-error -- Supabase generic chain infers Update as never
      .update({ following_count: Math.max(0, ((myProfile as any).following_count || 0) - 1) })
      .eq("id", user.id);
  }

  return NextResponse.json({ success: true, following: false });
}
