import type * as React from "react";
import { Input } from "@/components/ui/input";

export function TopBar({
  title,
  subtitle,
  searchValue,
  onSearchChange,
  children,
}: {
  title: string;
  subtitle?: string;
  searchValue?: string;
  onSearchChange?: (value: string) => void;
  children?: React.ReactNode;
}) {
  return (
    <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
      <div className="flex items-center justify-between gap-4 py-4">
        <div className="min-w-0">
          <h1 className="text-3xl font-semibold tracking-tight">{title}</h1>
          {subtitle ? <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p> : null}
        </div>
        <div className="flex items-center gap-3">
          {onSearchChange ? (
            <Input
              aria-label="Quick search"
              className="h-9 w-64"
              placeholder="Quick search"
              value={searchValue}
              onChange={(event) => onSearchChange(event.target.value)}
            />
          ) : null}
          {children}
        </div>
      </div>
    </header>
  );
}
