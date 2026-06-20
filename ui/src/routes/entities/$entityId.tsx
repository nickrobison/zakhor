import { createFileRoute } from "@tanstack/react-router";
import { EntityDetailPage } from "@/pages/EntityDetailPage";

export const Route = createFileRoute("/entities/$entityId")({
  component: EntityDetailPage,
});
