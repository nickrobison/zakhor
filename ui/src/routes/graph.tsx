import { createFileRoute } from "@tanstack/react-router";
import { GraphPage } from "@/pages/GraphPage";

export const Route = createFileRoute("/graph")({
  component: GraphPage,
});
