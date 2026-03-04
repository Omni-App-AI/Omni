import Link from "next/link";
import { Home, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { NotFoundParticleField } from "@/components/landing/NotFoundParticleField";
import { RouteSuggestions } from "@/components/landing/RouteSuggestions";

export default function NotFound() {
  return (
    <div className="relative min-h-[calc(100vh-180px)] overflow-hidden flex flex-col items-center justify-end pb-16 md:pb-24">
      <div className="absolute inset-0 gradient-mesh" />
      <div className="absolute inset-0 bg-grid fade-bottom" />
      <NotFoundParticleField />

      <div className="relative z-10 text-center px-4 max-w-lg">
        <p className="text-sm font-mono text-primary mb-3 tracking-wider">
          ERROR 404
        </p>
        <h1 className="text-3xl md:text-4xl font-bold tracking-tight mb-4">
          Page not found
        </h1>
        <p className="text-muted-foreground leading-relaxed mb-8">
          The page you&apos;re looking for doesn&apos;t exist or has been moved.
          Let&apos;s get you back on track.
        </p>
        <div className="flex items-center justify-center gap-3">
          <Link href="/">
            <Button size="lg">
              <Home className="h-4 w-4" />
              Back to home
            </Button>
          </Link>
          <Link href="/extensions">
            <Button size="lg" variant="outline">
              <Search className="h-4 w-4" />
              Browse extensions
            </Button>
          </Link>
        </div>
      </div>

      <RouteSuggestions />
    </div>
  );
}
