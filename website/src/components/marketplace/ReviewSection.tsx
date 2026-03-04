"use client";

import { useState } from "react";
import { Star } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Avatar } from "@/components/ui/avatar";
import { createClient } from "@/lib/supabase/client";
import { cn } from "@/lib/utils";

interface Review {
  id: string;
  rating: number;
  title: string | null;
  body: string | null;
  version: string | null;
  created_at: string;
  user: {
    username: string;
    display_name: string;
    avatar_url: string | null;
  };
}

interface ReviewSectionProps {
  extensionId: string;
  reviews: Review[];
  averageRating: number;
  reviewCount: number;
}

function StarRating({ rating, onRate, interactive = false }: {
  rating: number;
  onRate?: (r: number) => void;
  interactive?: boolean;
}) {
  return (
    <div className="flex gap-0.5">
      {[1, 2, 3, 4, 5].map((star) => (
        <Star
          key={star}
          className={cn(
            "h-5 w-5",
            star <= rating ? "fill-warning text-warning" : "text-muted-foreground",
            interactive && "cursor-pointer hover:text-warning",
          )}
          onClick={() => interactive && onRate?.(star)}
        />
      ))}
    </div>
  );
}

export function ReviewSection({ extensionId, reviews, averageRating, reviewCount }: ReviewSectionProps) {
  const [showForm, setShowForm] = useState(false);
  const [rating, setRating] = useState(0);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (rating === 0) return;
    setSubmitting(true);

    const supabase = createClient();
    const { data: { user } } = await supabase.auth.getUser();
    if (!user) return;

    await supabase.from("reviews").upsert({
      extension_id: extensionId,
      user_id: user.id,
      rating,
      title: title || null,
      body: body || null,
    } as any);

    setShowForm(false);
    setSubmitting(false);
    window.location.reload();
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-3">
            Reviews
            {reviewCount > 0 && (
              <span className="flex items-center gap-1 text-base font-normal text-muted-foreground">
                <Star className="h-4 w-4 fill-warning text-warning" />
                {averageRating.toFixed(1)} ({reviewCount})
              </span>
            )}
          </CardTitle>
          {!showForm && (
            <Button variant="outline" size="sm" onClick={() => setShowForm(true)}>
              Write a Review
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Review form */}
        {showForm && (
          <form onSubmit={handleSubmit} className="border border-border rounded-lg p-4 space-y-4">
            <div>
              <label className="text-sm font-medium mb-2 block">Your rating</label>
              <StarRating rating={rating} onRate={setRating} interactive />
            </div>
            <div>
              <label className="text-sm font-medium mb-1 block">Title (optional)</label>
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="Summarize your experience"
              />
            </div>
            <div>
              <label className="text-sm font-medium mb-1 block">Review (optional)</label>
              <Textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                placeholder="Tell others about your experience..."
                rows={3}
              />
            </div>
            <div className="flex gap-2">
              <Button type="submit" disabled={rating === 0 || submitting}>
                {submitting ? "Submitting..." : "Submit Review"}
              </Button>
              <Button type="button" variant="ghost" onClick={() => setShowForm(false)}>
                Cancel
              </Button>
            </div>
          </form>
        )}

        {/* Review list */}
        {reviews.length === 0 && !showForm ? (
          <p className="text-muted-foreground text-sm">No reviews yet. Be the first to review this extension.</p>
        ) : (
          <div className="space-y-4">
            {reviews.map((review) => (
              <div key={review.id} className="border-b border-border pb-4 last:border-0">
                <div className="flex items-center gap-3 mb-2">
                  <Avatar
                    src={review.user.avatar_url}
                    fallback={review.user.display_name}
                    size="sm"
                  />
                  <div>
                    <span className="text-sm font-medium">{review.user.display_name}</span>
                    <div className="flex items-center gap-2">
                      <StarRating rating={review.rating} />
                      <span className="text-xs text-muted-foreground">
                        {new Date(review.created_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                </div>
                {review.title && (
                  <h4 className="font-medium text-sm mb-1">{review.title}</h4>
                )}
                {review.body && (
                  <p className="text-sm text-muted-foreground">{review.body}</p>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
