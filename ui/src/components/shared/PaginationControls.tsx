import { Button } from "@/components/ui/button";

export function PaginationControls({
  currentPage,
  pageCount,
  onPageChange,
  pageSize,
  total,
}: {
  currentPage: number;
  pageCount: number;
  onPageChange: (page: number) => void;
  pageSize: number;
  total: number;
}) {
  const start = total > 0 ? (currentPage - 1) * pageSize + 1 : 0;
  const end = Math.min(currentPage * pageSize, total);

  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <p className="text-muted-foreground">
        Showing {start}-{end} of {total}
      </p>
      <div className="flex items-center gap-2">
        <Button type="button" variant="outline" size="sm" disabled={currentPage <= 1} onClick={() => onPageChange(currentPage - 1)}>
          Previous
        </Button>
        <span className="text-muted-foreground">
          Page {currentPage} of {pageCount}
        </span>
        <Button type="button" variant="outline" size="sm" disabled={currentPage >= pageCount} onClick={() => onPageChange(currentPage + 1)}>
          Next
        </Button>
      </div>
    </div>
  );
}
