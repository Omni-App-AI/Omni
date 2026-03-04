import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { REPUTATION_ACTIONS } from "@/lib/constants";
import { withProtection } from "@/lib/anti-bot/with-protection";

export const POST = withProtection(
  {
    rateLimit: "vote_cast",
    requireAuth: true,
  },
  async (_request, { user, body }) => {
    const supabase = await createClient();
    const { post_id, reply_id, value } = body as {
      post_id?: string;
      reply_id?: string;
      value?: number;
    };

    if (value !== 1 && value !== -1) {
      return NextResponse.json({ error: "Value must be 1 or -1" }, { status: 400 });
    }

    if (!post_id && !reply_id) {
      return NextResponse.json({ error: "Either post_id or reply_id is required" }, { status: 400 });
    }

    if (post_id && reply_id) {
      return NextResponse.json({ error: "Cannot vote on both post and reply" }, { status: 400 });
    }

    // Check for existing vote
    let existingQuery = supabase
      .from("forum_votes")
      .select("id, value")
      .eq("user_id", user.id);

    if (post_id) {
      existingQuery = existingQuery.eq("post_id", post_id);
    } else {
      existingQuery = existingQuery.eq("reply_id", reply_id!);
    }

    const { data: existingVotes } = await existingQuery;
    const existing = existingVotes && existingVotes.length > 0 ? existingVotes[0] : null;

    let scoreDelta = value!;
    let reputationDelta = 0;

    // Get the content author for reputation
    let contentAuthorId: string | null = null;
    if (post_id) {
      const { data: post } = await supabase.from("forum_posts").select("author_id").eq("id", post_id).single();
      contentAuthorId = post ? (post as any).author_id : null;
    } else {
      const { data: reply } = await supabase.from("forum_replies").select("author_id").eq("id", reply_id!).single();
      contentAuthorId = reply ? (reply as any).author_id : null;
    }

    // Don't allow self-voting
    if (contentAuthorId === user.id) {
      return NextResponse.json({ error: "Cannot vote on your own content" }, { status: 400 });
    }

    if (existing) {
      const existingValue = (existing as any).value as number;

      if (existingValue === value) {
        // Same vote -- toggle off (remove vote)
        await supabase.from("forum_votes").delete().eq("id", (existing as any).id);
        scoreDelta = -value!;
        reputationDelta = value === 1
          ? -(post_id ? REPUTATION_ACTIONS.post_upvoted : REPUTATION_ACTIONS.reply_upvoted)
          : -(post_id ? REPUTATION_ACTIONS.post_downvoted : REPUTATION_ACTIONS.reply_downvoted);
      } else {
        // Different vote -- switch direction
        // @ts-expect-error -- Supabase type inference limitation with manual Database type
        await supabase.from("forum_votes").update({ value }).eq("id", (existing as any).id);
        scoreDelta = value! * 2; // -1 to +1 = +2, or +1 to -1 = -2
        // Remove old reputation effect, add new
        if (value === 1) {
          // Was downvote, now upvote
          reputationDelta = (post_id ? REPUTATION_ACTIONS.post_upvoted : REPUTATION_ACTIONS.reply_upvoted)
            - (post_id ? REPUTATION_ACTIONS.post_downvoted : REPUTATION_ACTIONS.reply_downvoted);
        } else {
          // Was upvote, now downvote
          reputationDelta = (post_id ? REPUTATION_ACTIONS.post_downvoted : REPUTATION_ACTIONS.reply_downvoted)
            - (post_id ? REPUTATION_ACTIONS.post_upvoted : REPUTATION_ACTIONS.reply_upvoted);
        }
      }
    } else {
      // New vote
      // @ts-expect-error -- Supabase type inference limitation with manual Database type
      const { error } = await supabase.from("forum_votes").insert({
        user_id: user.id,
        post_id: post_id || null,
        reply_id: reply_id || null,
        value,
      });

      if (error) {
        return NextResponse.json({ error: error.message }, { status: 500 });
      }

      reputationDelta = value === 1
        ? (post_id ? REPUTATION_ACTIONS.post_upvoted : REPUTATION_ACTIONS.reply_upvoted)
        : (post_id ? REPUTATION_ACTIONS.post_downvoted : REPUTATION_ACTIONS.reply_downvoted);
    }

    // Use service_role for counter/reputation updates (protected by BEFORE UPDATE triggers)
    const service = createServiceClient();

    // Update score on post or reply
    if (post_id) {
      const { data: post } = await service.from("forum_posts").select("vote_score").eq("id", post_id).single();
      if (post) {
        await (service
          .from("forum_posts") as any)
          .update({ vote_score: ((post as any).vote_score || 0) + scoreDelta })
          .eq("id", post_id);
      }
    } else {
      const { data: reply } = await service.from("forum_replies").select("vote_score").eq("id", reply_id!).single();
      if (reply) {
        await (service
          .from("forum_replies") as any)
          .update({ vote_score: ((reply as any).vote_score || 0) + scoreDelta })
          .eq("id", reply_id!);
      }
    }

    // Update reputation on content author
    if (contentAuthorId && reputationDelta !== 0) {
      const { data: authorProfile } = await service
        .from("profiles")
        .select("reputation")
        .eq("id", contentAuthorId)
        .single();

      if (authorProfile) {
        const newRep = Math.max(0, ((authorProfile as any).reputation || 0) + reputationDelta);
        await (service
          .from("profiles") as any)
          .update({ reputation: newRep })
          .eq("id", contentAuthorId);
      }
    }

    return NextResponse.json({ success: true, score_delta: scoreDelta });
  }
);

export async function DELETE(request: NextRequest) {
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { searchParams } = new URL(request.url);
  const post_id = searchParams.get("post_id");
  const reply_id = searchParams.get("reply_id");

  let query = supabase.from("forum_votes").delete().eq("user_id", user.id);

  if (post_id) {
    query = query.eq("post_id", post_id);
  } else if (reply_id) {
    query = query.eq("reply_id", reply_id);
  } else {
    return NextResponse.json({ error: "post_id or reply_id required" }, { status: 400 });
  }

  const { error } = await query;

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ success: true });
}
