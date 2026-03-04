import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { REPUTATION_ACTIONS } from "@/lib/constants";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  // Check post ownership
  const { data: post } = await supabase
    .from("forum_posts")
    .select("id, author_id, accepted_reply_id")
    .eq("id", postId)
    .single();

  if (!post || (post as any).author_id !== user.id) {
    return NextResponse.json({ error: "Only the post author can accept an answer" }, { status: 403 });
  }

  const body = await request.json();
  const { reply_id } = body;

  if (!reply_id) {
    return NextResponse.json({ error: "reply_id is required" }, { status: 400 });
  }

  // Check reply exists and belongs to this post
  const { data: reply } = await supabase
    .from("forum_replies")
    .select("id, author_id, post_id")
    .eq("id", reply_id)
    .eq("post_id", postId)
    .single();

  if (!reply) {
    return NextResponse.json({ error: "Reply not found on this post" }, { status: 404 });
  }

  // Use service_role for is_accepted/reputation updates (protected by BEFORE UPDATE triggers)
  const service = createServiceClient();

  // Unaccept previous answer if any
  const prevAccepted = (post as any).accepted_reply_id;
  if (prevAccepted) {
    await (service
      .from("forum_replies") as any)
      .update({ is_accepted: false })
      .eq("id", prevAccepted);

    // Remove reputation from previous accepted author
    const { data: prevReply } = await service
      .from("forum_replies")
      .select("author_id")
      .eq("id", prevAccepted)
      .single();

    if (prevReply) {
      const prevAuthorId = (prevReply as any).author_id;
      const { data: prevProfile } = await service
        .from("profiles")
        .select("reputation")
        .eq("id", prevAuthorId)
        .single();

      if (prevProfile) {
        await (service
          .from("profiles") as any)
          .update({ reputation: Math.max(0, ((prevProfile as any).reputation || 0) - REPUTATION_ACTIONS.answer_accepted) })
          .eq("id", prevAuthorId);
      }
    }
  }

  // Accept new answer
  await (service
    .from("forum_replies") as any)
    .update({ is_accepted: true })
    .eq("id", reply_id);

  // Mark post as solved
  await (service
    .from("forum_posts") as any)
    .update({
      solved: true,
      accepted_reply_id: reply_id,
    })
    .eq("id", postId);

  // Award reputation to reply author
  const replyAuthorId = (reply as any).author_id;
  if (replyAuthorId !== user.id) {
    const { data: authorProfile } = await service
      .from("profiles")
      .select("reputation")
      .eq("id", replyAuthorId)
      .single();

    if (authorProfile) {
      await (service
        .from("profiles") as any)
        .update({
          reputation: ((authorProfile as any).reputation || 0) + REPUTATION_ACTIONS.answer_accepted,
        })
        .eq("id", replyAuthorId);
    }
  }

  return NextResponse.json({ success: true, accepted_reply_id: reply_id });
}
