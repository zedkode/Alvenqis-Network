interface PaginationProps {
  page: number;
  pageCount: number;
  onPageChange: (page: number) => void;
}

export function Pagination({ page, pageCount, onPageChange }: PaginationProps) {
  if (pageCount <= 1) {
    return null;
  }

  return (
    <div className="pagination" aria-label="Pagination">
      <button disabled={page === 1} onClick={() => onPageChange(page - 1)} type="button">
        Previous
      </button>
      <span>
        Page {page.toLocaleString()} of {pageCount.toLocaleString()}
      </span>
      <button
        disabled={page === pageCount}
        onClick={() => onPageChange(page + 1)}
        type="button"
      >
        Next
      </button>
    </div>
  );
}
