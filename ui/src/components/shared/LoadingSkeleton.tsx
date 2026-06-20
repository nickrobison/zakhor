import { Skeleton } from "@/components/ui/skeleton";

export function LoadingSkeleton({ count = 3, className = "h-12 w-full" }: { count?: number; className?: string }) {
  return (
    <div className="space-y-3" aria-label="Loading">
      {Array.from({ length: count }, (_, index) => (
        <span key={index} role="status" aria-label="Loading" data-testid="skeleton">
          <Skeleton className={className} />
        </span>
      ))}
    </div>
  );
}
