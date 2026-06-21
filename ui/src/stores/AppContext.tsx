import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

type AppContextValue = {
  sidebarOpen: boolean;
  setSidebarOpen: (open: boolean) => void;
};

const AppContext = createContext<AppContextValue | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const value = useMemo<AppContextValue>(() => ({ sidebarOpen, setSidebarOpen }), [sidebarOpen]);

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useAppContext() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useAppContext must be used within AppProvider");
  }
  return context;
}
