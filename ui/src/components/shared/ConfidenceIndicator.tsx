import { Progress } from "@/components/ui/progress";

export function ConfidenceIndicator({ value, label = "Confidence" }: { value: number; label?: string }) {
  const normalized = Math.min(100, Math.max(0, Math.round(value)));

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-sm">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-medium text-foreground">{normalized}%</span>
      </div>
      <Progress aria-label={`${label}: ${normalized}%`} value={normalized} />
    </div>
  );
}
