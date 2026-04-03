import { useEffect, useState } from "react";

import { TodoModal } from "./TodoModal";
import type {
  ClientLookupItem,
  TaskCollectionFilters,
  UserLookupItem,
} from "../lib/models";

type TaskFiltersModalProps = {
  isOpen: boolean;
  filters: TaskCollectionFilters;
  users: UserLookupItem[];
  clients: ClientLookupItem[];
  tracks: string[];
  onClose: () => void;
  onApply: (filters: TaskCollectionFilters) => void;
};

function emptyIfBlank(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

export function TaskFiltersModal({
  isOpen,
  filters,
  users,
  clients,
  tracks,
  onClose,
  onApply,
}: TaskFiltersModalProps) {
  const [draft, setDraft] = useState<TaskCollectionFilters>(filters);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setDraft(filters);
  }, [filters, isOpen]);

  return (
    <TodoModal title="Фильтры задач" isOpen={isOpen} onClose={onClose}>
      <form
        className="row g-3"
        onSubmit={(event) => {
          event.preventDefault();
          onApply(draft);
          onClose();
        }}
      >
        <div className="col-12">
          <label
            htmlFor="filterStatus"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Статус
          </label>
          <select
            id="filterStatus"
            className="form-select"
            value={draft.status ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                status: emptyIfBlank(event.target.value),
              }))
            }
          >
            <option value="">Все</option>
            <option value="Pending">В ожидании</option>
            <option value="InProgress">В работе</option>
            <option value="Blocked">Заблокирована</option>
            <option value="Completed">Завершена</option>
            <option value="Archived">Архивирована</option>
          </select>
        </div>
        <div className="col-12 col-md-6">
          <label
            htmlFor="filterTrack"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Трек
          </label>
          <input
            id="filterTrack"
            className="form-control"
            list="task-list-filter-tracks"
            value={draft.track ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                track: emptyIfBlank(event.target.value),
              }))
            }
          />
          <datalist id="task-list-filter-tracks">
            {tracks.map((track) => (
              <option value={track} key={track} />
            ))}
          </datalist>
        </div>
        <div className="col-12 col-md-6">
          <label
            htmlFor="filterPriority"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Приоритет
          </label>
          <select
            id="filterPriority"
            className="form-select"
            value={draft.priority ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                priority: emptyIfBlank(event.target.value),
              }))
            }
          >
            <option value="">Любой</option>
            <option value="Low">Низкий</option>
            <option value="Middle">Средний</option>
            <option value="High">Высокий</option>
          </select>
        </div>
        <div className="col-12 col-md-6">
          <label
            htmlFor="filterAssignee"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Исполнитель
          </label>
          <select
            id="filterAssignee"
            className="form-select"
            value={draft.assigneeId?.toString() ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                assigneeId: event.target.value
                  ? Number.parseInt(event.target.value, 10)
                  : undefined,
              }))
            }
          >
            <option value="">Любой</option>
            {users.map((user) => (
              <option value={user.id} key={user.id}>
                {user.name} ({user.email})
              </option>
            ))}
          </select>
        </div>
        <div className="col-12 col-md-6">
          <label
            htmlFor="filterClient"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Клиент
          </label>
          <select
            id="filterClient"
            className="form-select"
            value={draft.clientId?.toString() ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                clientId: event.target.value
                  ? Number.parseInt(event.target.value, 10)
                  : undefined,
              }))
            }
          >
            <option value="">Любой</option>
            {clients.map((client) => (
              <option value={client.id} key={client.id}>
                {client.name}
              </option>
            ))}
          </select>
        </div>
        <div className="col-12 col-md-6">
          <label
            htmlFor="filterUpdatedAfter"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Обновлена после
          </label>
          <input
            id="filterUpdatedAfter"
            type="date"
            className="form-control"
            value={draft.updatedAfter ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                updatedAfter: emptyIfBlank(event.target.value),
              }))
            }
          />
        </div>
        <div className="col-12 col-md-6">
          <label
            htmlFor="filterUpdatedBefore"
            className="form-label small text-uppercase text-muted mb-1"
          >
            Обновлена до
          </label>
          <input
            id="filterUpdatedBefore"
            type="date"
            className="form-control"
            value={draft.updatedBefore ?? ""}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                updatedBefore: emptyIfBlank(event.target.value),
              }))
            }
          />
        </div>
        <div className="col-12 d-flex flex-wrap gap-2 justify-content-end pt-3">
          <button type="submit" className="btn btn-primary">
            Применить
          </button>
          <button
            type="button"
            className="btn btn-outline-secondary"
            onClick={() => {
              onApply({});
              onClose();
            }}
          >
            Сбросить
          </button>
        </div>
      </form>
    </TodoModal>
  );
}
