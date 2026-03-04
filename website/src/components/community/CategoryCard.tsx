import Link from "next/link";
import { ArrowRight, Megaphone, HelpCircle, Sparkles, Lightbulb, Code2, MessageCircle } from "lucide-react";

const iconMap: Record<string, React.ComponentType<{ className?: string }>> = {
  Megaphone,
  HelpCircle,
  Sparkles,
  Lightbulb,
  Code2,
  MessageCircle,
};

interface CategoryCardProps {
  id: string;
  name: string;
  description: string;
  icon: string;
  postCount: number;
}

export function CategoryCard({ id, name, description, icon, postCount }: CategoryCardProps) {
  const Icon = iconMap[icon] || MessageCircle;

  return (
    <Link
      href={`/community/${id}`}
      className="group flex items-start gap-4 p-5 bg-card border border-border/50 rounded-lg hover:border-primary/30 hover:bg-card/80 transition-all"
    >
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary group-hover:bg-primary/20 transition-colors">
        <Icon className="h-5 w-5" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium group-hover:text-primary transition-colors">
            {name}
          </h3>
          <ArrowRight className="h-3.5 w-3.5 text-muted-foreground/50 group-hover:text-primary group-hover:translate-x-0.5 transition-all" />
        </div>
        <p className="text-xs text-muted-foreground mt-1 line-clamp-1">
          {description}
        </p>
        <p className="text-[10px] text-muted-foreground/60 mt-2">
          {postCount} {postCount === 1 ? "post" : "posts"}
        </p>
      </div>
    </Link>
  );
}
