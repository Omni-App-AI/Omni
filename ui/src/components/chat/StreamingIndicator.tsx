export function StreamingIndicator() {
  return (
    <div className="flex items-center gap-1.5 px-4 py-2">
      <span
        className="inline-block h-2 w-2 rounded-full bg-[var(--accent)] animate-[bounce-dot_1.4s_ease-in-out_infinite]"
        style={{ animationDelay: "0ms" }}
      />
      <span
        className="inline-block h-2 w-2 rounded-full bg-[var(--accent)] animate-[bounce-dot_1.4s_ease-in-out_infinite]"
        style={{ animationDelay: "200ms" }}
      />
      <span
        className="inline-block h-2 w-2 rounded-full bg-[var(--accent)] animate-[bounce-dot_1.4s_ease-in-out_infinite]"
        style={{ animationDelay: "400ms" }}
      />

      <style>{`
        @keyframes bounce-dot {
          0%, 80%, 100% {
            transform: translateY(0);
            opacity: 0.4;
          }
          40% {
            transform: translateY(-6px);
            opacity: 1;
          }
        }
      `}</style>
    </div>
  );
}
