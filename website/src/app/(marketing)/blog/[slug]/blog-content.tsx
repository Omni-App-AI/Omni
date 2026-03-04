"use client";

import ReactMarkdown from "react-markdown";

export function BlogContent({ body }: { body: string }) {
  return (
    <div className="prose prose-invert prose-headings:font-bold prose-headings:tracking-tight prose-a:text-primary prose-code:text-primary/80 prose-code:bg-secondary prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-code:before:content-[''] prose-code:after:content-[''] prose-pre:bg-secondary prose-pre:border prose-pre:border-border/50 prose-img:rounded-lg prose-img:border prose-img:border-border/50 max-w-none">
      <ReactMarkdown>{body}</ReactMarkdown>
    </div>
  );
}
