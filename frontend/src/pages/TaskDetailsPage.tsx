import { startTransition, useEffect, useState } from "react";

import { TaskEditModal } from "../components/TaskEditModal";
import { TaskEventTimeline } from "../components/TaskEventTimeline";
import { TaskPriorityBadge } from "../components/TaskPriorityBadge";
import { TaskStatusBadge } from "../components/TaskStatusBadge";
import { TodoModal } from "../components/TodoModal";
import { TodoShell } from "../components/TodoShell";
import { TodoShellFatalState } from "../components/TodoShellFatalState";
import {
  ApiMutationFailure,
  ApiResponseFailure,
  browserLocation,
  createTaskComment,
  deleteTask,
  fetchTaskDetails,
  updateTaskStatus,
} from "../lib/api";
import { renderMarkdownToHtml } from "../lib/markdown";
import type {
  ShellData,
  TaskClientSummary,
  TaskDetailsData,
  TaskUserSummary,
  UserMenuItem,
} from "../lib/models";
import { formatTaskDate, parseTaskIdFromPathname } from "../lib/taskDetails";
import { useTodoShell } from "../lib/useTodoShell";

type TaskDetailsLoadState =
  | { status: "loading"; previousData?: TaskDetailsData }
  | { status: "ready"; data: TaskDetailsData }
  | { status: "not-found" }
  | { status: "error"; message: string; previousData?: TaskDetailsData };

type TaskDetailsScreenProps = {
  shell: ShellData;
  fetchedMenuItems: UserMenuItem[];
  details: TaskDetailsData;
  isRefreshing: boolean;
  editOpen: boolean;
  completeOpen: boolean;
  deleteOpen: boolean;
  quickActionSubmitting: boolean;
  commentMarkdown: string;
  commentTab: "editor" | "preview";
  commentErrorMessage: string;
  commentFieldError?: string;
  commentSubmitting: boolean;
  completeComment: string;
  completeErrorMessage: string;
  completeSubmitting: boolean;
  deleteSubmitting: boolean;
  onOpenEdit: () => void;
  onCloseEdit: () => void;
  onRequestDelete: () => void;
  onCloseDelete: () => void;
  onConfirmDelete: () => void;
  onTakeInWork: () => void;
  onOpenComplete: () => void;
  onCloseComplete: () => void;
  onCommentChange: (value: string) => void;
  onCommentTabChange: (tab: "editor" | "preview") => void;
  onSubmitComment: () => void;
  onCompleteCommentChange: (value: string) => void;
  onSubmitComplete: () => void;
  onMutationSuccess: (message: string) => void;
};

function toFieldErrorMessage(
  fieldErrors: Array<{ field: string; message: string }>,
  fieldName: string,
) {
  return fieldErrors.find((fieldError) => fieldError.field === fieldName)
    ?.message;
}

function errorMessage(error: unknown, fallback: string) {
  if (error instanceof ApiMutationFailure) {
    return error.payload.message;
  }

  return error instanceof Error ? error.message : fallback;
}

function TaskUserValue({ user }: { user?: TaskUserSummary }) {
  if (!user) {
    return <span className="text-muted">—</span>;
  }

  return (
    <a
      role="button"
      tabIndex={0}
      data-bs-trigger="focus"
      data-bs-toggle="popover"
      title={user.name}
      data-bs-content={user.email}
      data-bs-original-title={user.name}
      aria-label={user.name}
    >
      <i className="bi bi-person-circle" />
      &nbsp;
      {user.name}
    </a>
  );
}

function TaskClientValue({ client }: { client?: TaskClientSummary }) {
  if (!client) {
    return <span className="text-muted">—</span>;
  }

  if (client.url) {
    return <a href={client.url}>{client.name}</a>;
  }

  return <span>{client.name}</span>;
}

export function TaskDetailsScreen({
  shell,
  fetchedMenuItems,
  details,
  isRefreshing,
  editOpen,
  completeOpen,
  deleteOpen,
  quickActionSubmitting,
  commentMarkdown,
  commentTab,
  commentErrorMessage,
  commentFieldError,
  commentSubmitting,
  completeComment,
  completeErrorMessage,
  completeSubmitting,
  deleteSubmitting,
  onOpenEdit,
  onCloseEdit,
  onRequestDelete,
  onCloseDelete,
  onConfirmDelete,
  onTakeInWork,
  onOpenComplete,
  onCloseComplete,
  onCommentChange,
  onCommentTabChange,
  onSubmitComment,
  onCompleteCommentChange,
  onSubmitComplete,
  onMutationSuccess,
}: TaskDetailsScreenProps) {
  const commentPreview = renderMarkdownToHtml(commentMarkdown);

  return (
    <TodoShell
      navigation={shell.navigation}
      currentUserEmail={shell.currentUser.email}
      homeUrl={shell.homeUrl}
      localMenuItems={shell.localMenuItems}
      fetchedMenuItems={fetchedMenuItems}
    >
      <main className="todo-shell-content">
        <div className="container my-3">
          {isRefreshing ? (
            <div className="alert alert-secondary py-2" role="status">
              Загрузка...
            </div>
          ) : null}

          <div className="card mb-3">
            <div className="card-header d-flex justify-content-between align-items-center">
              <h2 className="h5 mb-0">{details.task.title}</h2>
              <div className="d-flex align-items-center gap-2">
                <TaskStatusBadge status={details.task.status} />
                <button
                  type="button"
                  className="btn btn-outline-primary btn-sm"
                  onClick={onOpenEdit}
                  title="Редактировать задачу"
                >
                  <i className="bi bi-pencil-square" />
                </button>
              </div>
            </div>
            <div className="card-body">
              {details.task.status === "Pending" ||
              details.task.status === "InProgress" ? (
                <div className="mb-3">
                  {details.task.status === "Pending" ? (
                    <div className="d-flex gap-2 flex-wrap align-items-center">
                      <button
                        type="button"
                        className="btn btn-outline-primary btn-sm"
                        disabled={quickActionSubmitting}
                        onClick={onTakeInWork}
                      >
                        {quickActionSubmitting
                          ? "Сохранение..."
                          : "Взять в работу"}
                      </button>
                      <span className="text-muted small">
                        Статус перейдёт в «В работе»
                      </span>
                    </div>
                  ) : (
                    <button
                      type="button"
                      className="btn btn-success btn-sm"
                      disabled={quickActionSubmitting}
                      onClick={onOpenComplete}
                    >
                      Отметить как сделано
                    </button>
                  )}
                </div>
              ) : null}

              <div className="row mb-3">
                <div className="col-md-6">
                  <dl className="row mb-0">
                    <dt className="col-sm-4">Идентификатор</dt>
                    <dd className="col-sm-8">{details.task.id}</dd>
                    <dt className="col-sm-4">Трек</dt>
                    <dd className="col-sm-8">{details.task.track ?? "—"}</dd>
                    <dt className="col-sm-4">Срок</dt>
                    <dd className="col-sm-8">{details.task.dueDate ?? "—"}</dd>
                    <dt className="col-sm-4">Исполнитель</dt>
                    <dd className="col-sm-8">
                      <TaskUserValue user={details.assignee} />
                    </dd>
                    <dt className="col-sm-4">Клиент</dt>
                    <dd className="col-sm-8">
                      <TaskClientValue client={details.client} />
                    </dd>
                  </dl>
                </div>
                <div className="col-md-6">
                  <dl className="row mb-0">
                    <dt className="col-sm-4">Приоритет</dt>
                    <dd className="col-sm-8">
                      <TaskPriorityBadge priority={details.task.priority} />
                    </dd>
                    <dt className="col-sm-4">Создана</dt>
                    <dd className="col-sm-8">
                      {formatTaskDate(details.task.createdAt)}
                    </dd>
                    <dt className="col-sm-4">Обновлена</dt>
                    <dd className="col-sm-8">
                      {formatTaskDate(details.task.updatedAt)}
                    </dd>
                    <dt className="col-sm-4">Автор</dt>
                    <dd className="col-sm-8">
                      <TaskUserValue user={details.author} />
                    </dd>
                  </dl>
                </div>
              </div>

              <div className="row">
                <div className="col">
                  <h6>Описание:</h6>
                  {details.task.description ? (
                    <div
                      dangerouslySetInnerHTML={{
                        __html: details.task.description,
                      }}
                    />
                  ) : (
                    <span className="text-muted">—</span>
                  )}
                </div>
              </div>
            </div>
          </div>

          <div className="card mb-3">
            <div className="card-header">
              <h3 className="h6 mb-0">Добавить комментарий</h3>
            </div>
            <div className="card-body">
              {commentErrorMessage ? (
                <div className="alert alert-danger" role="alert">
                  {commentErrorMessage}
                </div>
              ) : null}
              <ul className="nav nav-tabs" role="tablist">
                <li className="nav-item" role="presentation">
                  <button
                    className={`nav-link ${commentTab === "editor" ? "active" : ""}`}
                    type="button"
                    onClick={() => onCommentTabChange("editor")}
                  >
                    Маркдаун
                  </button>
                </li>
                <li className="nav-item" role="presentation">
                  <button
                    className={`nav-link ${commentTab === "preview" ? "active" : ""}`}
                    type="button"
                    onClick={() => onCommentTabChange("preview")}
                  >
                    Превью
                  </button>
                </li>
              </ul>
              <div className="tab-content mb-1">
                <div
                  className={`tab-pane fade ${commentTab === "editor" ? "show active" : ""}`}
                  role="tabpanel"
                >
                  <textarea
                    className={`form-control border-top-0 rounded-top-0 ${
                      commentFieldError ? "is-invalid" : ""
                    }`}
                    rows={10}
                    value={commentMarkdown}
                    onChange={(event) => onCommentChange(event.target.value)}
                    placeholder="Содержание в формате markdown"
                  />
                  {commentFieldError ? (
                    <div className="invalid-feedback d-block">
                      {commentFieldError}
                    </div>
                  ) : null}
                </div>
                <div
                  className={`tab-pane fade ${commentTab === "preview" ? "show active" : ""}`}
                  role="tabpanel"
                >
                  <div
                    className="border border-top-0 rounded rounded-top-0 p-2 task-comment-preview"
                    dangerouslySetInnerHTML={{
                      __html:
                        commentPreview ||
                        "<span class='text-muted'>Нет содержимого.</span>",
                    }}
                  />
                </div>
              </div>
              <div className="d-flex justify-content-end gap-2">
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={commentSubmitting}
                  onClick={onSubmitComment}
                >
                  {commentSubmitting ? "Отправка..." : "Отправить"}
                </button>
              </div>
            </div>
          </div>

          <TaskEventTimeline events={details.events} />
        </div>
      </main>

      <TaskEditModal
        isOpen={editOpen}
        task={details.task}
        assignee={details.assignee}
        client={details.client}
        onClose={onCloseEdit}
        onRequestDelete={onRequestDelete}
        onMutationSuccess={onMutationSuccess}
      />

      <TodoModal
        title="Завершить задачу"
        isOpen={completeOpen}
        onClose={() => {
          if (!completeSubmitting) {
            onCloseComplete();
          }
        }}
      >
        {completeErrorMessage ? (
          <div className="alert alert-danger" role="alert">
            {completeErrorMessage}
          </div>
        ) : null}
        <p className="mb-2">
          Добавьте комментарий к завершению (необязательно).
        </p>
        <div className="mb-3">
          <label htmlFor="completeTaskComment" className="form-label">
            Комментарий
          </label>
          <textarea
            id="completeTaskComment"
            className="form-control"
            rows={3}
            value={completeComment}
            onChange={(event) => onCompleteCommentChange(event.target.value)}
            placeholder="Комментарий сохранится в истории задачи."
          />
        </div>
        <div className="d-flex justify-content-end gap-2">
          <button
            type="button"
            className="btn btn-outline-secondary"
            onClick={onCloseComplete}
            disabled={completeSubmitting}
          >
            Отмена
          </button>
          <button
            type="button"
            className="btn btn-success"
            onClick={onSubmitComplete}
            disabled={completeSubmitting}
          >
            {completeSubmitting ? "Сохранение..." : "Сохранить"}
          </button>
        </div>
      </TodoModal>

      <TodoModal
        title="Удалить задачу"
        isOpen={deleteOpen}
        onClose={() => {
          if (!deleteSubmitting) {
            onCloseDelete();
          }
        }}
      >
        <p className="mb-3">
          Задача <strong>{details.task.title}</strong> будет удалена без
          возможности восстановления.
        </p>
        <div className="d-flex justify-content-end gap-2">
          <button
            type="button"
            className="btn btn-outline-secondary"
            onClick={onCloseDelete}
            disabled={deleteSubmitting}
          >
            Отмена
          </button>
          <button
            type="button"
            className="btn btn-danger"
            onClick={onConfirmDelete}
            disabled={deleteSubmitting}
          >
            {deleteSubmitting ? "Удаление..." : "Удалить"}
          </button>
        </div>
      </TodoModal>
    </TodoShell>
  );
}

export function TaskDetailsPage() {
  const shellState = useTodoShell("Не удалось загрузить оболочку ToDo.");
  const [taskId, setTaskId] = useState(() =>
    parseTaskIdFromPathname(window.location.pathname),
  );
  const [detailsState, setDetailsState] = useState<TaskDetailsLoadState>(
    taskId ? { status: "loading" } : { status: "not-found" },
  );
  const [refreshToken, setRefreshToken] = useState(0);
  const [editOpen, setEditOpen] = useState(false);
  const [completeOpen, setCompleteOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [quickActionSubmitting, setQuickActionSubmitting] = useState(false);
  const [commentMarkdown, setCommentMarkdown] = useState("");
  const [commentTab, setCommentTab] = useState<"editor" | "preview">("editor");
  const [commentErrorMessage, setCommentErrorMessage] = useState("");
  const [commentFieldError, setCommentFieldError] = useState<
    string | undefined
  >(undefined);
  const [commentSubmitting, setCommentSubmitting] = useState(false);
  const [completeComment, setCompleteComment] = useState("");
  const [completeErrorMessage, setCompleteErrorMessage] = useState("");
  const [completeSubmitting, setCompleteSubmitting] = useState(false);
  const [deleteSubmitting, setDeleteSubmitting] = useState(false);

  useEffect(() => {
    const handlePopState = () => {
      startTransition(() => {
        setTaskId(parseTaskIdFromPathname(window.location.pathname));
      });
    };

    window.addEventListener("popstate", handlePopState);
    return () => {
      window.removeEventListener("popstate", handlePopState);
    };
  }, []);

  useEffect(() => {
    if (!taskId) {
      setDetailsState({ status: "not-found" });
      return;
    }

    let active = true;

    setDetailsState((current) =>
      current.status === "ready"
        ? { status: "loading", previousData: current.data }
        : current.status === "error" && current.previousData
          ? { status: "loading", previousData: current.previousData }
          : { status: "loading" },
    );

    void fetchTaskDetails(taskId)
      .then((data) => {
        if (!active) {
          return;
        }

        setDetailsState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        if (error instanceof ApiResponseFailure && error.status === 404) {
          setDetailsState({ status: "not-found" });
          return;
        }

        setDetailsState((current) => ({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить задачу.",
          previousData:
            current.status === "loading" ? current.previousData : undefined,
        }));
      });

    return () => {
      active = false;
    };
  }, [refreshToken, taskId]);

  useEffect(() => {
    if (detailsState.status === "error" && detailsState.previousData != null) {
      window.showFlashMessage?.(detailsState.message, "danger");
    }
  }, [detailsState]);

  if (shellState.status === "error") {
    return <TodoShellFatalState message={shellState.message} />;
  }

  if (
    shellState.status === "loading" ||
    (detailsState.status === "loading" && detailsState.previousData == null)
  ) {
    return null;
  }

  const details =
    detailsState.status === "ready"
      ? detailsState.data
      : detailsState.status === "loading" || detailsState.status === "error"
        ? detailsState.previousData
        : undefined;

  if (detailsState.status === "error" && !details) {
    return <TodoShellFatalState message={detailsState.message} />;
  }

  if (!details) {
    return (
      <TodoShell
        navigation={shellState.shell.navigation}
        currentUserEmail={shellState.shell.currentUser.email}
        homeUrl={shellState.shell.homeUrl}
        localMenuItems={shellState.shell.localMenuItems}
        fetchedMenuItems={shellState.authMenuItems}
      >
        <main className="container py-5 todo-shell-content">
          <div className="alert alert-warning mb-0" role="alert">
            Задача не найдена.
          </div>
        </main>
      </TodoShell>
    );
  }

  const refreshDetails = (message: string) => {
    window.showFlashMessage?.(message, "primary");
    setRefreshToken((current) => current + 1);
  };

  return (
    <TaskDetailsScreen
      shell={shellState.shell}
      fetchedMenuItems={shellState.authMenuItems}
      details={details}
      isRefreshing={detailsState.status === "loading"}
      editOpen={editOpen}
      completeOpen={completeOpen}
      deleteOpen={deleteOpen}
      quickActionSubmitting={quickActionSubmitting}
      commentMarkdown={commentMarkdown}
      commentTab={commentTab}
      commentErrorMessage={commentErrorMessage}
      commentFieldError={commentFieldError}
      commentSubmitting={commentSubmitting}
      completeComment={completeComment}
      completeErrorMessage={completeErrorMessage}
      completeSubmitting={completeSubmitting}
      deleteSubmitting={deleteSubmitting}
      onOpenEdit={() => setEditOpen(true)}
      onCloseEdit={() => setEditOpen(false)}
      onRequestDelete={() => {
        setEditOpen(false);
        setDeleteOpen(true);
      }}
      onCloseDelete={() => setDeleteOpen(false)}
      onConfirmDelete={() => {
        setDeleteSubmitting(true);

        void deleteTask(details.task.id)
          .then((response) => {
            browserLocation.assign(response.redirectTo ?? "/");
          })
          .catch((error) => {
            setDeleteSubmitting(false);
            setDeleteOpen(false);
            window.showFlashMessage?.(
              errorMessage(error, "Не удалось удалить задачу."),
              "danger",
            );
          });
      }}
      onTakeInWork={() => {
        const form = new URLSearchParams();
        form.set("status", "InProgress");
        form.set("assign_self", "true");

        setQuickActionSubmitting(true);

        void updateTaskStatus(details.task.id, form)
          .then((response) => {
            refreshDetails(response.message);
          })
          .catch((error) => {
            window.showFlashMessage?.(
              errorMessage(error, "Не удалось обновить статус задачи."),
              "danger",
            );
          })
          .finally(() => {
            setQuickActionSubmitting(false);
          });
      }}
      onOpenComplete={() => {
        setCompleteErrorMessage("");
        setCompleteOpen(true);
      }}
      onCloseComplete={() => {
        setCompleteOpen(false);
        setCompleteComment("");
        setCompleteErrorMessage("");
      }}
      onCommentChange={(value) => {
        setCommentMarkdown(value);
        setCommentErrorMessage("");
        setCommentFieldError(undefined);
      }}
      onCommentTabChange={setCommentTab}
      onSubmitComment={() => {
        if (!commentMarkdown.trim()) {
          setCommentErrorMessage("Ошибка валидации формы.");
          setCommentFieldError("Введите комментарий.");
          return;
        }

        const form = new URLSearchParams();
        form.set("message", renderMarkdownToHtml(commentMarkdown));

        setCommentSubmitting(true);
        setCommentErrorMessage("");
        setCommentFieldError(undefined);

        void createTaskComment(details.task.id, form)
          .then((response) => {
            setCommentMarkdown("");
            setCommentTab("editor");
            refreshDetails(response.message);
          })
          .catch((error) => {
            if (error instanceof ApiMutationFailure) {
              setCommentErrorMessage(error.payload.message);
              setCommentFieldError(
                toFieldErrorMessage(error.payload.fieldErrors, "message"),
              );
              return;
            }

            setCommentErrorMessage(
              errorMessage(error, "Не удалось добавить комментарий."),
            );
          })
          .finally(() => {
            setCommentSubmitting(false);
          });
      }}
      onCompleteCommentChange={(value) => {
        setCompleteComment(value);
        setCompleteErrorMessage("");
      }}
      onSubmitComplete={() => {
        const form = new URLSearchParams();
        form.set("status", "Completed");
        if (completeComment.trim()) {
          form.set("comment", renderMarkdownToHtml(completeComment));
        }

        setCompleteSubmitting(true);
        setCompleteErrorMessage("");

        void updateTaskStatus(details.task.id, form)
          .then((response) => {
            setCompleteComment("");
            setCompleteOpen(false);
            refreshDetails(response.message);
          })
          .catch((error) => {
            setCompleteErrorMessage(
              errorMessage(error, "Не удалось обновить статус задачи."),
            );
          })
          .finally(() => {
            setCompleteSubmitting(false);
          });
      }}
      onMutationSuccess={(message) => {
        refreshDetails(message);
      }}
    />
  );
}
