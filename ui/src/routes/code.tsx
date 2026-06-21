import { createFileRoute } from "@tanstack/react-router";
import { CodePage } from "@/pages/CodePage";

export const Route = createFileRoute("/code")({
  component: CodePage,
});
