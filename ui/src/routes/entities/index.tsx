import { createFileRoute } from "@tanstack/react-router";
import { EntitiesPage } from "@/pages/EntitiesPage";

export const Route = createFileRoute("/entities/")({
  component: EntitiesPage,
});
