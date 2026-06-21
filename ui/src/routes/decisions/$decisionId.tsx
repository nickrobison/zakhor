import { createFileRoute } from "@tanstack/react-router";
import { DecisionDetailPage } from "@/pages/DecisionDetailPage";

export const Route = createFileRoute("/decisions/$decisionId")({
  component: DecisionDetailPage,
});
