import { useDeferredValue, useEffect, useId, useState } from "react";

import { TodoModal } from "./TodoModal";
import {
  ApiMutationFailure,
  fetchClients,
  fetchTracks,
  fetchUsers,
  updateTask,
} from "../lib/api";
import { renderMarkdownToHtml } from "../lib/markdown";
import type {
  TaskClientSummary,
  TaskDetailsTask,
  TaskUserSummary,
} from "../lib/models";

type AssigneeOption = {
  name: string;
  email: string;
};

type ClientOption = {
  name: string;
  publicId: string;
};

type FieldErrors = Record<string, string>;

type TaskEditModalProps = {
  isOpen: boolean;
  task: TaskDetailsTask;
  assignee?: TaskUserSummary;
  client?: TaskClientSummary;
  onClose: () => void;
  onMutationSuccess: (message: string) => void;
  onRequestDelete: () => void;
};

function toFieldErrorMap(
  fieldErrors: Array<{ field: string; message: string }>,
): FieldErrors {
  return Object.fromEntries(
    fieldErrors.map((fieldError) => [fieldError.field, fieldError.message]),
  );
}

function formatAssigneeOption(option: AssigneeOption) {
  return `${option.name} (${option.email})`;
}

function formatClientOption(option: ClientOption) {
  return `${option.name} (${option.publicId})`;
}

function mergeAssigneeOptions(
  selectedAssignee: AssigneeOption | undefined,
  options: AssigneeOption[],
) {
  const byEmail = new Map<string, AssigneeOption>();
  if (selectedAssignee) {
    byEmail.set(selectedAssignee.email, selectedAssignee);
  }
  for (const option of options) {
    byEmail.set(option.email, option);
  }
  return Array.from(byEmail.values());
}

function mergeClientOptions(
  selectedClient: ClientOption | undefined,
  options: ClientOption[],
) {
  const byPublicId = new Map<string, ClientOption>();
  if (selectedClient) {
    byPublicId.set(selectedClient.publicId, selectedClient);
  }
  for (const option of options) {
    byPublicId.set(option.publicId, option);
  }
  return Array.from(byPublicId.values());
}

function selectedAssigneeFromValue(
  value: string,
  options: AssigneeOption[],
  current?: AssigneeOption,
) {
  const trimmed = value.trim();
  if (!trimmed) {
    return undefined;
  }

  const currentLabel = current ? formatAssigneeOption(current) : undefined;
  if (current && trimmed === currentLabel) {
    return current;
  }

  return options.find((option) => formatAssigneeOption(option) === trimmed);
}

function selectedClientFromValue(
  value: string,
  options: ClientOption[],
  current?: ClientOption,
) {
  const trimmed = value.trim();
  if (!trimmed) {
    return undefined;
  }

  const currentLabel = current ? formatClientOption(current) : undefined;
  if (current && trimmed === currentLabel) {
    return current;
  }

  return options.find((option) => formatClientOption(option) === trimmed);
}

export function TaskEditModal({
  isOpen,
  task,
  assignee,
  client,
  onClose,
  onMutationSuccess,
  onRequestDelete,
}: TaskEditModalProps) {
  const [title, setTitle] = useState(task.title);
  const [markdown, setMarkdown] = useState(task.description ?? "");
  const [activeTab, setActiveTab] = useState<"editor" | "preview">("editor");
  const [dueDate, setDueDate] = useState(task.dueDate ?? "");
  const [status, setStatus] = useState(task.status);
  const [track, setTrack] = useState(task.track ?? "");
  const [priority, setPriority] = useState(task.priority);
  const [assigneeInput, setAssigneeInput] = useState("");
  const [selectedAssignee, setSelectedAssignee] = useState<
    AssigneeOption | undefined
  >(undefined);
  const [assigneeOptions, setAssigneeOptions] = useState<AssigneeOption[]>([]);
  const [assigneeLookupError, setAssigneeLookupError] = useState("");
  const [clientInput, setClientInput] = useState("");
  const [selectedClient, setSelectedClient] = useState<
    ClientOption | undefined
  >(undefined);
  const [clientOptions, setClientOptions] = useState<ClientOption[]>([]);
  const [clientLookupError, setClientLookupError] = useState("");
  const [trackSuggestions, setTrackSuggestions] = useState<string[]>([]);
  const [trackLookupError, setTrackLookupError] = useState("");
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [errorMessage, setErrorMessage] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const deferredAssigneeInput = useDeferredValue(assigneeInput);
  const deferredClientInput = useDeferredValue(clientInput);
  const assigneeListId = useId().replaceAll(":", "");
  const clientListId = useId().replaceAll(":", "");
  const trackListId = useId().replaceAll(":", "");

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const nextAssignee = assignee
      ? { name: assignee.name, email: assignee.email }
      : undefined;
    const nextClient = client
      ? { name: client.name, publicId: client.publicId }
      : undefined;

    setTitle(task.title);
    setMarkdown(task.description ?? "");
    setActiveTab("editor");
    setDueDate(task.dueDate ?? "");
    setStatus(task.status);
    setTrack(task.track ?? "");
    setPriority(task.priority);
    setSelectedAssignee(nextAssignee);
    setAssigneeInput(nextAssignee ? formatAssigneeOption(nextAssignee) : "");
    setAssigneeOptions(nextAssignee ? [nextAssignee] : []);
    setAssigneeLookupError("");
    setSelectedClient(nextClient);
    setClientInput(nextClient ? formatClientOption(nextClient) : "");
    setClientOptions(nextClient ? [nextClient] : []);
    setClientLookupError("");
    setFieldErrors({});
    setErrorMessage("");
    setIsSubmitting(false);
  }, [assignee, client, isOpen, task]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    let active = true;

    void fetchTracks()
      .then((tracks) => {
        if (!active) {
          return;
        }

        setTrackSuggestions(tracks.map((trackItem) => trackItem.value));
        setTrackLookupError("");
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setTrackLookupError(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить треки.",
        );
      });

    return () => {
      active = false;
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const query = deferredAssigneeInput.trim();
    const selectedLabel = selectedAssignee
      ? formatAssigneeOption(selectedAssignee)
      : undefined;

    if (!query || query === selectedLabel) {
      setAssigneeLookupError("");
      setAssigneeOptions(
        selectedAssignee ? mergeAssigneeOptions(selectedAssignee, []) : [],
      );
      return;
    }

    let active = true;

    void fetchUsers(query)
      .then((users) => {
        if (!active) {
          return;
        }

        const options = users.map((user) => ({
          name: user.name,
          email: user.email,
        }));
        setAssigneeLookupError("");
        setAssigneeOptions(mergeAssigneeOptions(selectedAssignee, options));
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setAssigneeLookupError(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить исполнителей.",
        );
      });

    return () => {
      active = false;
    };
  }, [deferredAssigneeInput, isOpen, selectedAssignee]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const query = deferredClientInput.trim();
    const selectedLabel = selectedClient
      ? formatClientOption(selectedClient)
      : undefined;

    if (!query || query === selectedLabel) {
      setClientLookupError("");
      setClientOptions(
        selectedClient ? mergeClientOptions(selectedClient, []) : [],
      );
      return;
    }

    let active = true;

    void fetchClients(query)
      .then((clientsLookup) => {
        if (!active) {
          return;
        }

        const options = clientsLookup.map((clientItem) => ({
          name: clientItem.name,
          publicId: clientItem.publicId,
        }));
        setClientLookupError("");
        setClientOptions(mergeClientOptions(selectedClient, options));
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setClientLookupError(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить клиентов.",
        );
      });

    return () => {
      active = false;
    };
  }, [deferredClientInput, isOpen, selectedClient]);

  const markdownHtml = renderMarkdownToHtml(markdown);
  const assigneeFieldError = fieldErrors.name ?? fieldErrors.email;
  const clientFieldError =
    fieldErrors.client_name ?? fieldErrors.client_public_id;

  return (
    <TodoModal
      title="Изменить задачу"
      isOpen={isOpen}
      onClose={() => {
        if (!isSubmitting) {
          onClose();
        }
      }}
      dialogClassName="modal-xl"
      footer={
        <button
          className="btn btn-danger"
          type="button"
          disabled={isSubmitting}
          onClick={onRequestDelete}
        >
          Удалить
        </button>
      }
    >
      <form
        onSubmit={async (event) => {
          event.preventDefault();

          const nextFieldErrors: FieldErrors = {};
          if (assigneeInput.trim() && !selectedAssignee) {
            nextFieldErrors.name = "Выберите исполнителя из списка.";
          }
          if (clientInput.trim() && !selectedClient) {
            nextFieldErrors.client_name = "Выберите клиента из списка.";
          }

          if (Object.keys(nextFieldErrors).length > 0) {
            setFieldErrors(nextFieldErrors);
            setErrorMessage("Ошибка валидации формы.");
            return;
          }

          const form = new URLSearchParams();
          form.set("title", title);
          form.set("status", status);
          form.set("priority", priority);
          form.set("due_date", dueDate);
          form.set("track", track);
          form.set("message", markdown.trim() ? markdownHtml : "");

          if (selectedAssignee) {
            form.set("name", selectedAssignee.name);
            form.set("email", selectedAssignee.email);
          }

          if (selectedClient) {
            form.set("client_name", selectedClient.name);
            form.set("client_public_id", selectedClient.publicId);
          }

          setIsSubmitting(true);
          setFieldErrors({});
          setErrorMessage("");

          try {
            const response = await updateTask(task.id, form);
            onMutationSuccess(response.message);
            onClose();
          } catch (error) {
            if (error instanceof ApiMutationFailure) {
              setErrorMessage(error.payload.message);
              setFieldErrors(toFieldErrorMap(error.payload.fieldErrors));
            } else {
              setErrorMessage(
                error instanceof Error
                  ? error.message
                  : "Не удалось обновить задачу.",
              );
            }
          } finally {
            setIsSubmitting(false);
          }
        }}
      >
        {errorMessage ? (
          <div className="alert alert-danger" role="alert">
            {errorMessage}
          </div>
        ) : null}

        <div className="row mb-3">
          <label htmlFor="taskEditTitle" className="col-md-2 col-form-label">
            Название
          </label>
          <div className="col-md-10">
            <input
              id="taskEditTitle"
              className={`form-control ${fieldErrors.title ? "is-invalid" : ""}`}
              name="title"
              type="text"
              value={title}
              required
              onChange={(event) => setTitle(event.target.value)}
            />
            {fieldErrors.title ? (
              <div className="invalid-feedback d-block">
                {fieldErrors.title}
              </div>
            ) : null}
          </div>
        </div>

        <div className="mb-3">
          <ul className="nav nav-tabs" role="tablist">
            <li className="nav-item" role="presentation">
              <button
                className={`nav-link ${activeTab === "editor" ? "active" : ""}`}
                type="button"
                onClick={() => setActiveTab("editor")}
              >
                Маркдаун
              </button>
            </li>
            <li className="nav-item" role="presentation">
              <button
                className={`nav-link ${activeTab === "preview" ? "active" : ""}`}
                type="button"
                onClick={() => setActiveTab("preview")}
              >
                Превью
              </button>
            </li>
          </ul>
          <div className="border border-top-0 rounded-bottom p-3">
            {activeTab === "editor" ? (
              <>
                <textarea
                  className={`form-control ${fieldErrors.message ? "is-invalid" : ""}`}
                  rows={10}
                  value={markdown}
                  onChange={(event) => setMarkdown(event.target.value)}
                  placeholder="Содержание в формате markdown"
                />
                {fieldErrors.message ? (
                  <div className="invalid-feedback d-block">
                    {fieldErrors.message}
                  </div>
                ) : null}
              </>
            ) : (
              <div
                className="add-task-preview"
                dangerouslySetInnerHTML={{
                  __html:
                    markdownHtml ||
                    "<span class='text-muted'>Нет содержимого.</span>",
                }}
              />
            )}
          </div>
        </div>

        <div className="row mb-3">
          <label htmlFor="taskEditDueDate" className="col-md-2 col-form-label">
            Срок
          </label>
          <div className="col-md-10">
            <div className="input-group">
              <input
                id="taskEditDueDate"
                className={`form-control ${
                  fieldErrors.due_date ? "is-invalid" : ""
                }`}
                type="date"
                value={dueDate}
                onChange={(event) => setDueDate(event.target.value)}
              />
              <button
                className="btn btn-outline-secondary"
                type="button"
                onClick={() => setDueDate("")}
              >
                Очистить
              </button>
            </div>
            {fieldErrors.due_date ? (
              <div className="invalid-feedback d-block">
                {fieldErrors.due_date}
              </div>
            ) : (
              <div className="form-text">
                Оставьте поле пустым, чтобы не задавать срок.
              </div>
            )}
          </div>
        </div>

        <div className="row mb-3">
          <label htmlFor="taskEditStatus" className="col-md-2 col-form-label">
            Статус
          </label>
          <div className="col-md-10">
            <select
              id="taskEditStatus"
              className={`form-select ${fieldErrors.status ? "is-invalid" : ""}`}
              value={status}
              onChange={(event) => setStatus(event.target.value)}
            >
              <option value="Pending">В ожидании</option>
              <option value="InProgress">В работе</option>
              <option value="Blocked">Заблокирована</option>
              <option value="Completed">Завершена</option>
              <option value="Archived">Архивирована</option>
            </select>
            {fieldErrors.status ? (
              <div className="invalid-feedback d-block">
                {fieldErrors.status}
              </div>
            ) : null}
          </div>
        </div>

        <div className="row mb-3">
          <label htmlFor="taskEditTrack" className="col-md-2 col-form-label">
            Трек
          </label>
          <div className="col-md-10">
            <input
              id="taskEditTrack"
              className={`form-control ${fieldErrors.track ? "is-invalid" : ""}`}
              list={trackListId}
              type="text"
              value={track}
              onChange={(event) => setTrack(event.target.value)}
              placeholder="Трек"
            />
            <datalist id={trackListId}>
              {trackSuggestions.map((trackSuggestion) => (
                <option key={trackSuggestion} value={trackSuggestion} />
              ))}
            </datalist>
            {fieldErrors.track ? (
              <div className="invalid-feedback d-block">
                {fieldErrors.track}
              </div>
            ) : trackLookupError ? (
              <div className="form-text text-danger">{trackLookupError}</div>
            ) : null}
          </div>
        </div>

        <div className="row mb-3">
          <label htmlFor="taskEditPriority" className="col-md-2 col-form-label">
            Приоритет
          </label>
          <div className="col-md-10">
            <select
              id="taskEditPriority"
              className={`form-select ${
                fieldErrors.priority ? "is-invalid" : ""
              }`}
              value={priority}
              onChange={(event) => setPriority(event.target.value)}
            >
              <option value="Middle">Средний</option>
              <option value="Low">Низкий</option>
              <option value="High">Высокий</option>
            </select>
            {fieldErrors.priority ? (
              <div className="invalid-feedback d-block">
                {fieldErrors.priority}
              </div>
            ) : null}
          </div>
        </div>

        <div className="row mb-3">
          <label htmlFor="taskEditAssignee" className="col-md-2 col-form-label">
            Исполнитель
          </label>
          <div className="col-md-10">
            <input
              id="taskEditAssignee"
              className={`form-control ${assigneeFieldError ? "is-invalid" : ""}`}
              list={assigneeListId}
              type="text"
              value={assigneeInput}
              placeholder="Поиск исполнителя"
              onChange={(event) => {
                const value = event.target.value;
                setAssigneeInput(value);
                setFieldErrors((current) => {
                  const next = { ...current };
                  delete next.name;
                  delete next.email;
                  return next;
                });
                setSelectedAssignee(
                  selectedAssigneeFromValue(
                    value,
                    assigneeOptions,
                    selectedAssignee,
                  ),
                );
              }}
            />
            <datalist id={assigneeListId}>
              {assigneeOptions.map((option) => (
                <option
                  key={option.email}
                  value={formatAssigneeOption(option)}
                />
              ))}
            </datalist>
            {assigneeFieldError ? (
              <div className="invalid-feedback d-block">
                {assigneeFieldError}
              </div>
            ) : assigneeLookupError ? (
              <div className="form-text text-danger">{assigneeLookupError}</div>
            ) : null}
          </div>
        </div>

        <div className="row mb-3">
          <label htmlFor="taskEditClient" className="col-md-2 col-form-label">
            Клиент
          </label>
          <div className="col-md-10">
            <input
              id="taskEditClient"
              className={`form-control ${clientFieldError ? "is-invalid" : ""}`}
              list={clientListId}
              type="text"
              value={clientInput}
              placeholder="Поиск клиента"
              onChange={(event) => {
                const value = event.target.value;
                setClientInput(value);
                setFieldErrors((current) => {
                  const next = { ...current };
                  delete next.client_name;
                  delete next.client_public_id;
                  return next;
                });
                setSelectedClient(
                  selectedClientFromValue(value, clientOptions, selectedClient),
                );
              }}
            />
            <datalist id={clientListId}>
              {clientOptions.map((option) => (
                <option
                  key={option.publicId}
                  value={formatClientOption(option)}
                />
              ))}
            </datalist>
            {clientFieldError ? (
              <div className="invalid-feedback d-block">{clientFieldError}</div>
            ) : clientLookupError ? (
              <div className="form-text text-danger">{clientLookupError}</div>
            ) : null}
          </div>
        </div>

        <div className="row">
          <div className="col">
            <button
              className="btn btn-primary"
              type="submit"
              disabled={isSubmitting}
            >
              {isSubmitting ? "Сохранение..." : "Сохранить"}
            </button>
          </div>
        </div>
      </form>
    </TodoModal>
  );
}
