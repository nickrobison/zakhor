import { defaultNavItems, SidebarNavigation, type NavItem } from "@/components/shared/SidebarNavigation";
import { TopBar } from "@/components/shared/TopBar";

export function LayoutShell({
  children,
  navItems = defaultNavItems,
  title = "Zakhor",
  subtitle,
  quickSearchValue,
  onQuickSearchChange,
  topbarActions,
}: {
  children: React.ReactNode;
  navItems?: readonly NavItem[];
  title?: string;
  subtitle?: string;
  quickSearchValue?: string;
  onQuickSearchChange?: (value: string) => void;
  topbarActions?: React.ReactNode;
}) {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="flex min-h-screen">
        <SidebarNavigation items={navItems} />
        <div className="flex min-w-0 flex-1 flex-col">
          <TopBar
            title={title}
            subtitle={subtitle}
            searchValue={quickSearchValue}
            onSearchChange={onQuickSearchChange}
          >
            {topbarActions}
          </TopBar>
          <main className="flex-1 p-6 md:p-8">{children}</main>
        </div>
      </div>
    </div>
  );
}
