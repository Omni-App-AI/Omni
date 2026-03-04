import { type ReactNode } from "react";

interface LayoutProps {
  sidebar: ReactNode;
  children: ReactNode;
}

export function Layout({ sidebar, children }: LayoutProps) {
  return (
    <div className="h-full flex bg-[var(--bg-primary)] text-[var(--text-primary)]">
      {sidebar}
      <main className="flex-1 flex flex-col overflow-hidden">{children}</main>
    </div>
  );
}
