import { Outlet, createRootRoute } from "@tanstack/react-router";
import { ReactFlowProvider } from "@xyflow/react";
import { LayoutShell } from "@/components/shared/LayoutShell";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return (
    <ReactFlowProvider>
      <LayoutShell title="Zakhor" subtitle="Knowledge graph memory console">
        <Outlet />
      </LayoutShell>
    </ReactFlowProvider>
  );
}
