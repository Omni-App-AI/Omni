import { Heart } from "lucide-react";

export function ThankYouBanner() {
  return (
    <div className="border-t border-success/20 bg-success/5">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-4">
        <div className="flex items-center justify-center gap-2 text-sm text-success">
          <Heart className="h-4 w-4" />
          <span className="font-medium">
            Thank you for your donation! Your support helps keep Omni free and open source.
          </span>
        </div>
      </div>
    </div>
  );
}
