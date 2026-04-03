type TaskPriorityBadgeProps = {
  priority?: string;
};

function priorityPresentation(priority?: string) {
  switch (priority) {
    case "Low":
      return {
        label: "Низкий",
        className: "badge border border-primary text-primary",
        iconClass: "bi bi-arrow-down",
      };
    case "Middle":
      return {
        label: "Средний",
        className: "badge border border-warning text-warning",
        iconClass: "bi bi-arrow-right",
      };
    case "High":
      return {
        label: "Высокий",
        className: "badge border border-danger text-danger",
        iconClass: "bi bi-arrow-up",
      };
    default:
      return {
        label: priority ?? "—",
        className: "badge border border-secondary text-secondary",
        iconClass: "",
      };
  }
}

export function TaskPriorityBadge({ priority }: TaskPriorityBadgeProps) {
  if (!priority) {
    return <span className="text-muted">—</span>;
  }

  const presentation = priorityPresentation(priority);

  return (
    <span className={presentation.className}>
      {presentation.iconClass ? (
        <i className={`${presentation.iconClass} me-1`} />
      ) : null}
      {presentation.label}
    </span>
  );
}
