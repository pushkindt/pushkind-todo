import { useDeferredValue, useEffect, useState } from "react";

import { TodoModal } from "./TodoModal";
import {
  createTask,
  fetchUsers,
  isApiMutationError,
  toFieldErrorMap,
  uploadTasks,
} from "../lib/api";
import {
  MarkdownComposer,
  renderMarkdownToHtml,
} from "@pushkind/frontend-shell/markdown";

type AssigneeOption = {
  name: string;
  email: string;
};

type AddTaskModalProps = {
  isOpen: boolean;
  trackSuggestions: string[];
  filesServiceUrl: string;
  prefillAssignee?: AssigneeOption;
  onClose: () => void;
  onMutationSuccess: (message: string) => void;
};

export function unmatchedAssigneeError(
  assigneeInput: string,
  selectedAssignee?: AssigneeOption,
) {
  return assigneeInput.trim() && !selectedAssignee
    ? "Выберите исполнителя из списка."
    : undefined;
}

function formatAssigneeOption(option: AssigneeOption) {
  return `${option.name} (${option.email})`;
}

function emptyIfBlank(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

export function AddTaskModal({
  isOpen,
  trackSuggestions,
  filesServiceUrl,
  prefillAssignee,
  onClose,
  onMutationSuccess,
}: AddTaskModalProps) {
  const [title, setTitle] = useState("");
  const [track, setTrack] = useState("");
  const [priority, setPriority] = useState("Middle");
  const [markdown, setMarkdown] = useState("");
  const [assigneeInput, setAssigneeInput] = useState("");
  const [selectedAssignee, setSelectedAssignee] = useState<
    AssigneeOption | undefined
  >(undefined);
  const [assigneeOptions, setAssigneeOptions] = useState<AssigneeOption[]>([]);
  const [assigneeLookupError, setAssigneeLookupError] = useState("");
  const [taskFieldErrors, setTaskFieldErrors] = useState<
    Record<string, string>
  >({});
  const [taskErrorMessage, setTaskErrorMessage] = useState("");
  const [taskSubmitting, setTaskSubmitting] = useState(false);
  const [uploadErrorMessage, setUploadErrorMessage] = useState("");
  const [uploadFieldErrors, setUploadFieldErrors] = useState<
    Record<string, string>
  >({});
  const [uploadSubmitting, setUploadSubmitting] = useState(false);
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const deferredAssigneeInput = useDeferredValue(assigneeInput);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setTitle("");
    setTrack("");
    setPriority("Middle");
    setMarkdown("");
    setTaskFieldErrors({});
    setTaskErrorMessage("");
    setUploadFieldErrors({});
    setUploadErrorMessage("");
    setUploadFile(null);

    if (prefillAssignee) {
      setSelectedAssignee(prefillAssignee);
      setAssigneeInput(formatAssigneeOption(prefillAssignee));
      setAssigneeOptions([prefillAssignee]);
    } else {
      setSelectedAssignee(undefined);
      setAssigneeInput("");
      setAssigneeOptions([]);
    }
  }, [isOpen, prefillAssignee]);

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
      setAssigneeOptions(selectedAssignee ? [selectedAssignee] : []);
      return;
    }

    let active = true;

    void fetchUsers(query)
      .then((users) => {
        if (!active) {
          return;
        }

        setAssigneeLookupError("");
        setAssigneeOptions(
          users.map((user) => ({
            name: user.name,
            email: user.email,
          })),
        );
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

  const markdownHtml = renderMarkdownToHtml(markdown);

  return (
    <TodoModal
      title="Добавить задачу"
      isOpen={isOpen}
      onClose={() => {
        if (!taskSubmitting && !uploadSubmitting) {
          onClose();
        }
      }}
      dialogClassName="modal-lg"
      footer={
        <form
          className="w-100"
          onSubmit={async (event) => {
            event.preventDefault();

            if (!uploadFile) {
              setUploadFieldErrors({ csv: "Выберите CSV-файл." });
              setUploadErrorMessage("Ошибка валидации формы.");
              return;
            }

            const form = new FormData();
            form.set("csv", uploadFile);

            setUploadSubmitting(true);
            setUploadFieldErrors({});
            setUploadErrorMessage("");

            try {
              const response = await uploadTasks(form);
              onMutationSuccess(response.message);
              onClose();
            } catch (error) {
              if (isApiMutationError(error)) {
                setUploadErrorMessage(error.message);
                setUploadFieldErrors(toFieldErrorMap(error));
              } else {
                setUploadErrorMessage(
                  error instanceof Error
                    ? error.message
                    : "Не удалось загрузить CSV-файл.",
                );
              }
            } finally {
              setUploadSubmitting(false);
            }
          }}
        >
          <div className="row g-2 align-items-center">
            <div className="col">
              <input
                className={`form-control ${
                  uploadFieldErrors.csv ? "is-invalid" : ""
                }`}
                type="file"
                accept=".csv"
                onChange={(event) =>
                  setUploadFile(event.target.files?.item(0) ?? null)
                }
              />
              {uploadFieldErrors.csv ? (
                <div className="invalid-feedback d-block">
                  {uploadFieldErrors.csv}
                </div>
              ) : null}
            </div>
            <div className="col-auto">
              <button
                className="btn btn-success"
                type="submit"
                disabled={uploadSubmitting}
              >
                {uploadSubmitting ? "Загрузка..." : "Загрузить CSV"}
              </button>
            </div>
          </div>
          <div className="form-text">
            <sup>
              <small className="text-muted">
                Ожидаются столбцы <code>title</code> и опционально{" "}
                <code>description</code>
              </small>
            </sup>
          </div>
          {uploadErrorMessage ? (
            <div className="alert alert-danger py-2 mb-0 mt-3" role="alert">
              {uploadErrorMessage}
            </div>
          ) : null}
        </form>
      }
    >
      <form
        onSubmit={async (event) => {
          event.preventDefault();

          const assigneeError = unmatchedAssigneeError(
            assigneeInput,
            selectedAssignee,
          );
          if (assigneeError) {
            setTaskFieldErrors({ name: assigneeError });
            setTaskErrorMessage("Ошибка валидации формы.");
            return;
          }

          const form = new URLSearchParams();
          form.set("title", title);
          form.set("priority", priority);

          const normalizedTrack = emptyIfBlank(track);
          if (normalizedTrack) {
            form.set("track", normalizedTrack);
          }

          if (markdown.trim()) {
            form.set("message", markdownHtml);
          }

          if (selectedAssignee) {
            form.set("name", selectedAssignee.name);
            form.set("email", selectedAssignee.email);
          }

          setTaskSubmitting(true);
          setTaskFieldErrors({});
          setTaskErrorMessage("");

          try {
            const response = await createTask(form);
            onMutationSuccess(response.message);
            onClose();
          } catch (error) {
            if (isApiMutationError(error)) {
              setTaskErrorMessage(error.message);
              setTaskFieldErrors(toFieldErrorMap(error));
            } else {
              setTaskErrorMessage(
                error instanceof Error
                  ? error.message
                  : "Не удалось добавить задачу.",
              );
            }
          } finally {
            setTaskSubmitting(false);
          }
        }}
      >
        {taskErrorMessage ? (
          <div className="alert alert-danger" role="alert">
            {taskErrorMessage}
          </div>
        ) : null}
        <div className="row mb-3">
          <label htmlFor="taskModalTitle" className="col-md-2 col-form-label">
            Название
          </label>
          <div className="col-md-10">
            <input
              name="title"
              type="text"
              className={`form-control ${
                taskFieldErrors.title ? "is-invalid" : ""
              }`}
              id="taskModalTitle"
              placeholder="Название"
              value={title}
              required
              onChange={(event) => setTitle(event.target.value)}
            />
            {taskFieldErrors.title ? (
              <div className="invalid-feedback d-block">
                {taskFieldErrors.title}
              </div>
            ) : null}
          </div>
        </div>
        <div className="row mb-3">
          <label htmlFor="taskModalTrack" className="col-md-2 col-form-label">
            Трек
          </label>
          <div className="col-md-10">
            <input
              name="track"
              list="available-tracks"
              type="text"
              className={`form-control ${
                taskFieldErrors.track ? "is-invalid" : ""
              }`}
              id="taskModalTrack"
              placeholder="Трек"
              value={track}
              onChange={(event) => setTrack(event.target.value)}
            />
            <datalist id="available-tracks">
              {trackSuggestions.map((trackOption) => (
                <option value={trackOption} key={trackOption} />
              ))}
            </datalist>
            {taskFieldErrors.track ? (
              <div className="invalid-feedback d-block">
                {taskFieldErrors.track}
              </div>
            ) : null}
          </div>
        </div>
        <div className="row mb-3">
          <label
            htmlFor="taskModalPriority"
            className="col-md-2 col-form-label"
          >
            Приоритет
          </label>
          <div className="col-md-10">
            <select
              name="priority"
              className={`form-select ${
                taskFieldErrors.priority ? "is-invalid" : ""
              }`}
              id="taskModalPriority"
              value={priority}
              required
              onChange={(event) => setPriority(event.target.value)}
            >
              <option value="Middle">Средний</option>
              <option value="Low">Низкий</option>
              <option value="High">Высокий</option>
            </select>
            {taskFieldErrors.priority ? (
              <div className="invalid-feedback d-block">
                {taskFieldErrors.priority}
              </div>
            ) : null}
          </div>
        </div>
        <div className="mb-3">
          <MarkdownComposer
            value={markdown}
            onChange={setMarkdown}
            rows={10}
            placeholder="Содержание в формате markdown"
            textareaClassName={
              taskFieldErrors.message ? "is-invalid" : undefined
            }
            previewClassName="add-task-preview"
            editorLabel="Маркдаун"
            previewLabel="Превью"
            fileBrowserLabel="Файлы"
            emptyPreviewLabel="Превью будет показано здесь."
            fileBrowser={
              filesServiceUrl
                ? {
                    baseUrl: filesServiceUrl,
                    helpText:
                      "Загрузите или найдите файл, скопируйте ссылку и вставьте её в markdown как ссылку или изображение.",
                  }
                : undefined
            }
          />
          {taskFieldErrors.message ? (
            <div className="invalid-feedback d-block">
              {taskFieldErrors.message}
            </div>
          ) : null}
        </div>
        <div className="row mb-3">
          <label htmlFor="user-add-form-id" className="col-md-2 col-form-label">
            Исполнитель
          </label>
          <div className="col-md-10">
            <input
              className={`form-control my-1 ${
                taskFieldErrors.name || taskFieldErrors.email
                  ? "is-invalid"
                  : ""
              }`}
              id="user-add-form-id"
              list="available-assignees"
              placeholder="Поиск исполнителя"
              value={assigneeInput}
              onChange={(event) => {
                const nextValue = event.target.value;
                setAssigneeInput(nextValue);
                setTaskFieldErrors((current) => {
                  const nextErrors = { ...current };
                  delete nextErrors.name;
                  delete nextErrors.email;
                  return nextErrors;
                });

                const matchedOption = assigneeOptions.find(
                  (option) => formatAssigneeOption(option) === nextValue,
                );

                if (matchedOption) {
                  setSelectedAssignee(matchedOption);
                } else if (!nextValue.trim()) {
                  setSelectedAssignee(undefined);
                } else {
                  setSelectedAssignee(undefined);
                }
              }}
            />
            <datalist id="available-assignees">
              {assigneeOptions.map((option) => (
                <option
                  value={formatAssigneeOption(option)}
                  key={option.email}
                />
              ))}
            </datalist>
            {taskFieldErrors.name ? (
              <div className="invalid-feedback d-block">
                {taskFieldErrors.name}
              </div>
            ) : null}
            {taskFieldErrors.email ? (
              <div className="invalid-feedback d-block">
                {taskFieldErrors.email}
              </div>
            ) : null}
            <div className="form-text">
              {selectedAssignee ? (
                <>
                  Текущий исполнитель: {selectedAssignee.name} (
                  {selectedAssignee.email}). Очистите поле, чтобы снять
                  назначение.
                </>
              ) : (
                <>
                  Укажите пользователя, чтобы назначить задачу, или оставьте
                  поле пустым.
                </>
              )}
            </div>
            {assigneeLookupError ? (
              <div className="text-danger small mt-1">
                {assigneeLookupError}
              </div>
            ) : null}
          </div>
        </div>
        <div className="row mb-0">
          <div className="col">
            <button
              className="btn btn-primary"
              type="submit"
              disabled={taskSubmitting}
            >
              {taskSubmitting ? "Сохранение..." : "Сохранить"}
            </button>
          </div>
        </div>
      </form>
    </TodoModal>
  );
}
