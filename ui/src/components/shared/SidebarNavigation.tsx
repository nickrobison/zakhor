import { Link } from "@tanstack/react-router";
import { cn } from "@/lib/utils";

export const defaultNavItems = [
  { label: "Home", to: "/" },
  { label: "Search", to: "/search" },
  { label: "Decisions", to: "/decisions" },
  { label: "Entities", to: "/entities" },
  { label: "Observations", to: "/observations" },
  { label: "Graph", to: "/graph" },
  { label: "Code", to: "/code" },
  { label: "Admin", to: "/admin" },
] as const;

export type NavItem = (typeof defaultNavItems)[number];

export function SidebarNavigation({ items = defaultNavItems }: { items?: readonly NavItem[] }) {
  return (
    <aside className="flex h-full w-64 flex-col border-r bg-card p-4">
      <Link to="/" className="mb-6 block text-xl font-semibold tracking-tight">
        Zakhor
      </Link>
      <nav className="flex flex-col gap-1" aria-label="Primary navigation">
        {items.map((item) => (
          <Link
            key={item.to}
            to={item.to}
            className={cn(
              "rounded-md px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
            )}
            activeProps={{ className: "bg-accent font-medium text-foreground" }}
          >
            {item.label}
          </Link>
        ))}
      </nav>
    </aside>
  );
}
