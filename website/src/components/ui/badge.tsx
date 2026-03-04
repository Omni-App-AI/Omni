import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center border px-2 py-0.5 text-xs font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
  {
    variants: {
      variant: {
        default: "border-primary/20 bg-primary/10 text-primary rounded-md",
        secondary: "border-border bg-secondary text-muted-foreground rounded-md",
        destructive: "border-destructive/20 bg-destructive/10 text-destructive rounded-md",
        outline: "text-foreground border-border rounded-md",
        success: "border-success/20 bg-success/10 text-success rounded-md",
        warning: "border-warning/20 bg-warning/10 text-warning rounded-md",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
