type TaskStatusBadgeProps = {
  status?: string;
};

function statusPresentation(status?: string) {
  switch (status) {
    case "Pending":
      return {
        label: "В ожидании",
        className: "badge border border-secondary text-secondary",
        iconClass: "bi bi-question-circle",
      };
    case "InProgress":
      return {
        label: "В работе",
        className: "badge border border-primary text-primary",
        iconClass: "bi bi-clock",
      };
    case "Blocked":
      return {
        label: "Заблокирована",
        className: "badge border border-danger text-danger",
        iconClass: "bi bi-ban",
      };
    case "Completed":
      return {
        label: "Завершена",
        className: "badge border border-success text-success",
        iconClass: "bi bi-check2-circle",
      };
    case "Archived":
      return {
        label: "Архивирована",
        className: "badge border border-dark text-dark",
        iconClass: "bi bi-archive",
      };
    default:
      return {
        label: status ?? "—",
        className: "badge border border-secondary text-secondary",
        iconClass: "",
      };
  }
}

export function TaskStatusBadge({ status }: TaskStatusBadgeProps) {
  if (!status) {
    return <span className="text-muted">—</span>;
  }

  const presentation = statusPresentation(status);

  return (
    <span className={presentation.className}>
      {presentation.iconClass ? (
        <i className={`${presentation.iconClass} me-1`} />
      ) : null}
      {presentation.label}
    </span>
  );
}
