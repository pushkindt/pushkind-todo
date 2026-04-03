import type { ReactNode } from "react";

import { TaskPriorityBadge } from "./TaskPriorityBadge";
import { TaskStatusBadge } from "./TaskStatusBadge";
import type { TaskEventItem, TaskUserSummary } from "../lib/models";
import { formatTaskDateTime } from "../lib/taskDetails";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readOptionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value : undefined;
}

function eventTypeLabel(eventType: string) {
  switch (eventType) {
    case "Comment":
      return { label: "Комментарий", className: "badge text-bg-info" };
    case "StatusChanged":
      return { label: "Изменение статуса", className: "badge text-bg-primary" };
    case "AssignmentChanged":
      return {
        label: "Изменение исполнителя",
        className: "badge text-bg-success",
      };
    case "MetadataUpdated":
      return {
        label: "Обновление данных",
        className: "badge text-bg-warning",
      };
    default:
      return {
        label: eventType || "Неизвестное событие",
        className: "badge text-bg-secondary",
      };
  }
}

function metadataLabel(key: string) {
  switch (key) {
    case "title":
      return "Название";
    case "description":
      return "Описание";
    case "due_date":
      return "Срок";
    case "completed_at":
      return "Завершение";
    case "priority":
      return "Приоритет";
    case "track":
      return "Трек";
    case "client":
    case "client_id":
      return "Клиент";
    default:
      return key.replaceAll("_", " ");
  }
}

function UserLabel({ user }: { user?: TaskUserSummary }) {
  if (!user) {
    return <span className="text-muted">—</span>;
  }

  return (
    <span title={user.email}>
      <i className="bi bi-person-circle me-1" />
      {user.name}
    </span>
  );
}

function AssignmentEventUser({ value }: { value: unknown }) {
  if (!isRecord(value)) {
    return <span className="text-muted">—</span>;
  }

  const user: TaskUserSummary = {
    id: typeof value.id === "number" ? value.id : 0,
    name: readOptionalString(value.name) ?? "—",
    email: readOptionalString(value.email) ?? "",
  };

  return <UserLabel user={user.email ? user : undefined} />;
}

function MetadataValue({ field, value }: { field: string; value: unknown }) {
  const stringValue = readOptionalString(value);

  if (field === "priority") {
    return stringValue ? (
      <TaskPriorityBadge priority={stringValue} />
    ) : (
      <span className="text-muted">—</span>
    );
  }

  if (field === "description") {
    return stringValue ? (
      <div
        className="small"
        dangerouslySetInnerHTML={{ __html: stringValue }}
      />
    ) : (
      <span className="text-muted">—</span>
    );
  }

  if (field === "completed_at") {
    return <span>{formatTaskDateTime(stringValue)}</span>;
  }

  return <span>{stringValue ?? "—"}</span>;
}

function renderEventData(event: TaskEventItem): ReactNode {
  if (!isRecord(event.eventData)) {
    return (
      <pre className="mb-0 small">
        {JSON.stringify(event.eventData, null, 2)}
      </pre>
    );
  }

  switch (event.eventType) {
    case "Comment": {
      const text = readOptionalString(event.eventData.text);
      return text ? (
        <div dangerouslySetInnerHTML={{ __html: text }} />
      ) : (
        <span className="text-muted">Комментарий без текста.</span>
      );
    }
    case "StatusChanged": {
      const previous = readOptionalString(event.eventData.from);
      const current = readOptionalString(event.eventData.to);
      return (
        <div className="d-flex flex-column gap-1">
          <div>
            <span className="text-muted me-2">Было:</span>
            <TaskStatusBadge status={previous} />
          </div>
          <div>
            <span className="text-muted me-2">Стало:</span>
            <TaskStatusBadge status={current} />
          </div>
        </div>
      );
    }
    case "AssignmentChanged":
      return (
        <div className="d-flex flex-column gap-2">
          <div>
            <div className="text-muted small">Было</div>
            <AssignmentEventUser value={event.eventData.from} />
          </div>
          <div>
            <div className="text-muted small">Стало</div>
            <AssignmentEventUser value={event.eventData.to} />
          </div>
        </div>
      );
    case "MetadataUpdated":
      return (
        <div className="vstack gap-2">
          {Object.entries(event.eventData).length > 0 ? (
            Object.entries(event.eventData).map(([field, change]) => {
              const currentChange = isRecord(change) ? change : {};

              return (
                <div key={field}>
                  <div className="fw-semibold">{metadataLabel(field)}</div>
                  <div className="text-muted small">Было</div>
                  <div className="mb-1">
                    <MetadataValue field={field} value={currentChange.from} />
                  </div>
                  <div className="text-muted small">Стало</div>
                  <div>
                    <MetadataValue field={field} value={currentChange.to} />
                  </div>
                </div>
              );
            })
          ) : (
            <span className="text-muted">Изменения не найдены.</span>
          )}
        </div>
      );
    default:
      return (
        <pre className="mb-0 small">
          {JSON.stringify(event.eventData, null, 2)}
        </pre>
      );
  }
}

export function TaskEventTimeline({ events }: { events: TaskEventItem[] }) {
  return (
    <div className="card">
      <div className="card-header">
        <h3 className="h6 mb-0">История событий</h3>
      </div>
      <div className="list-group list-group-flush">
        {events.length > 0 ? (
          events.map((event) => {
            const presentation = eventTypeLabel(event.eventType);

            return (
              <div className="list-group-item task-event-item" key={event.id}>
                <div className="d-flex justify-content-between gap-3">
                  <div>
                    <span className={presentation.className}>
                      {presentation.label}
                    </span>
                  </div>
                  <small className="text-muted">
                    {formatTaskDateTime(event.createdAt)}
                  </small>
                </div>
                <div className="mt-2">
                  <strong>Автор события:</strong>{" "}
                  <UserLabel user={event.author} />
                </div>
                <div className="mt-2 mb-0 bg-light p-2 rounded task-event-payload">
                  {renderEventData(event)}
                </div>
              </div>
            );
          })
        ) : (
          <div className="list-group-item">
            <p className="mb-0 text-muted">Для этой задачи пока нет событий.</p>
          </div>
        )}
      </div>
    </div>
  );
}
