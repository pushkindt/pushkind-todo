type PageToken = number | "ellipsis";

type TaskListPaginationProps = {
  page: number;
  totalPages: number;
  onSelectPage: (page: number) => void;
};

function buildPaginationTokens(page: number, totalPages: number): PageToken[] {
  if (totalPages <= 1) {
    return [];
  }

  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, index) => index + 1);
  }

  const tokens: PageToken[] = [1];
  const windowStart = Math.max(2, page - 1);
  const windowEnd = Math.min(totalPages - 1, page + 1);

  if (windowStart > 2) {
    tokens.push("ellipsis");
  }

  for (let current = windowStart; current <= windowEnd; current += 1) {
    tokens.push(current);
  }

  if (windowEnd < totalPages - 1) {
    tokens.push("ellipsis");
  }

  tokens.push(totalPages);
  return tokens;
}

export function TaskListPagination({
  page,
  totalPages,
  onSelectPage,
}: TaskListPaginationProps) {
  const tokens = buildPaginationTokens(page, totalPages);
  if (tokens.length === 0) {
    return null;
  }

  return (
    <nav aria-label="pagination">
      <ul className="pagination justify-content-center flex-wrap mb-0">
        {tokens.map((token, index) =>
          token === "ellipsis" ? (
            <li className="page-item" key={`ellipsis-${index}`}>
              <span className="page-link">…</span>
            </li>
          ) : token === page ? (
            <li className="page-item active" aria-current="page" key={token}>
              <span className="page-link">{token}</span>
            </li>
          ) : (
            <li className="page-item" key={token}>
              <button
                type="button"
                className="page-link"
                onClick={() => onSelectPage(token)}
              >
                {token}
              </button>
            </li>
          ),
        )}
      </ul>
    </nav>
  );
}
