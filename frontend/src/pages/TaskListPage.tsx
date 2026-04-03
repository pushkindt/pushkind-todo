import { startTransition, useEffect, useState } from "react";

import { AddTaskModal } from "../components/AddTaskModal";
import { TaskFiltersModal } from "../components/TaskFiltersModal";
import { TaskListPagination } from "../components/TaskListPagination";
import { TaskPriorityBadge } from "../components/TaskPriorityBadge";
import { TaskStatusBadge } from "../components/TaskStatusBadge";
import { TodoShell } from "../components/TodoShell";
import { TodoShellFatalState } from "../components/TodoShellFatalState";
import { fetchTaskCollection } from "../lib/api";
import type {
  ShellData,
  TaskCollectionData,
  TaskCollectionFilters,
  TaskListItem,
  UserMenuItem,
} from "../lib/models";
import {
  buildTaskCollectionParams,
  buildTaskListSearch,
  hasActiveTaskFilters,
  parseTaskListQuery,
  readTaskAssigneePrefill,
  stripTaskTransientParams,
  type TaskAssigneePrefill,
} from "../lib/taskListQuery";
import { useTodoShell } from "../lib/useTodoShell";

type CollectionLoadState =
  | { status: "loading"; previousData?: TaskCollectionData }
  | { status: "ready"; data: TaskCollectionData }
  | { status: "error"; message: string; previousData?: TaskCollectionData };

type TaskListSearchFormProps = {
  searchValue?: string;
  onSubmit: (search: string | undefined) => void;
};

type TaskListScreenProps = {
  shell: ShellData;
  fetchedMenuItems: UserMenuItem[];
  collection: TaskCollectionData;
  isRefreshing: boolean;
  filtersOpen: boolean;
  addTaskOpen: boolean;
  prefillAssignee?: TaskAssigneePrefill;
  onSearchSubmit: (search: string | undefined) => void;
  onOpenFilters: () => void;
  onCloseFilters: () => void;
  onApplyFilters: (filters: TaskCollectionFilters) => void;
  onOpenAddTask: () => void;
  onCloseAddTask: () => void;
  onMutationSuccess: (message: string) => void;
  onSelectPage: (page: number) => void;
};

function emptyIfBlank(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function formatUpdatedAt(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  const hours = `${date.getHours()}`.padStart(2, "0");
  const minutes = `${date.getMinutes()}`.padStart(2, "0");

  return `${year}-${month}-${day} ${hours}:${minutes}`;
}

function TaskListSearchForm({
  searchValue,
  onSubmit,
}: TaskListSearchFormProps) {
  const [draft, setDraft] = useState(searchValue ?? "");

  useEffect(() => {
    setDraft(searchValue ?? "");
  }, [searchValue]);

  return (
    <form
      className="d-flex w-100"
      role="search"
      action="/"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(emptyIfBlank(draft));
      }}
    >
      <div className="input-group me-2">
        <input
          name="search"
          type="search"
          className="form-control"
          aria-label="Search"
          placeholder="Поиск"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button
          type="submit"
          className="btn btn-outline-secondary"
          aria-label="Найти"
        >
          <i className="bi bi-search" />
        </button>
      </div>
    </form>
  );
}

function TaskListItemRow({
  item,
  isRecentlyUpdated,
}: {
  item: TaskListItem;
  isRecentlyUpdated: boolean;
}) {
  return (
    <a
      className={`list-group-item py-3 selectable text-reset text-decoration-none ${
        isRecentlyUpdated ? "task-recent" : ""
      }`}
      href={`/task/${item.id}`}
    >
      <div className="row align-items-center">
        <div className="col-12 col-lg-4">
          <div className="d-flex align-items-center gap-2 mb-1">
            <span className="badge text-bg-secondary">{item.track ?? "—"}</span>
            <span className="fw-semibold">{item.title}</span>
          </div>
          <div className="text-muted small mb-1">
            <i className="bi bi-clock me-1" />
            {formatUpdatedAt(item.updatedAt)}
          </div>
          <div className="text-muted small">
            <span className="fw-semibold">Исполнитель: </span>
            {item.assignee ? (
              <span title={item.assignee.email}>
                <i className="bi bi-person-circle me-1" />
                {item.assignee.name}
              </span>
            ) : (
              <span className="text-muted">—</span>
            )}
          </div>
        </div>
        <div className="col-12 col-lg-4">
          {item.description ? (
            <div
              className="content-truncate"
              dangerouslySetInnerHTML={{ __html: item.description }}
            />
          ) : (
            <div className="content-truncate">—</div>
          )}
        </div>
        <div className="col-12 col-lg-4 text-lg-end mt-2 mt-lg-0">
          <div className="d-flex d-lg-block gap-2 justify-content-start justify-content-lg-end">
            <TaskStatusBadge status={item.status} />
            <TaskPriorityBadge priority={item.priority} />
          </div>
        </div>
      </div>
    </a>
  );
}

export function TaskListScreen({
  shell,
  fetchedMenuItems,
  collection,
  isRefreshing,
  filtersOpen,
  addTaskOpen,
  prefillAssignee,
  onSearchSubmit,
  onOpenFilters,
  onCloseFilters,
  onApplyFilters,
  onOpenAddTask,
  onCloseAddTask,
  onMutationSuccess,
  onSelectPage,
}: TaskListScreenProps) {
  const activeFilters = hasActiveTaskFilters({
    ...collection.activeFilters,
    page: collection.pagination.page,
  });

  return (
    <TodoShell
      navigation={shell.navigation}
      currentUserEmail={shell.currentUser.email}
      homeUrl={shell.homeUrl}
      localMenuItems={shell.localMenuItems}
      fetchedMenuItems={fetchedMenuItems}
      search={
        <TaskListSearchForm
          searchValue={collection.activeFilters.search}
          onSubmit={onSearchSubmit}
        />
      }
    >
      <main className="todo-shell-content">
        <div className="container bg-white border rounded my-2 task-list-page-shell">
          <div className="row">
            <div className="col text-center add-task-container">
              <button
                className="btn btn-link"
                type="button"
                onClick={onOpenAddTask}
              >
                <i className="bi bi-plus-circle" />
              </button>
            </div>
            <div className="col-auto">
              <button
                className="btn btn-sm btn-outline-secondary d-flex align-items-center gap-2 mt-1"
                type="button"
                onClick={onOpenFilters}
              >
                <i className="bi bi-funnel" />
                <span
                  className={`badge text-bg-primary ${
                    activeFilters ? "" : "d-none"
                  }`}
                >
                  •
                </span>
              </button>
            </div>
          </div>

          {isRefreshing ? (
            <div className="alert alert-secondary py-2" role="status">
              Загрузка...
            </div>
          ) : null}

          <div className="row d-none d-lg-flex fw-bold">
            <div className="col-lg-4 overflow-hidden px-3">Название</div>
            <div className="col-lg-4 overflow-hidden px-3">Описание</div>
            <div className="col-lg-4 overflow-hidden text-end px-3">Статус</div>
          </div>

          <div className="list-group">
            {collection.items.length > 0 ? (
              collection.items.map((item) => (
                <TaskListItemRow
                  item={item}
                  key={item.id}
                  isRecentlyUpdated={collection.recentlyUpdatedTaskIds.includes(
                    item.id,
                  )}
                />
              ))
            ) : (
              <div className="alert alert-warning my-2" role="alert">
                Нет задач для отображения.
              </div>
            )}
          </div>

          <div className="pt-3 pb-2">
            <TaskListPagination
              page={collection.pagination.page}
              totalPages={collection.pagination.totalPages}
              onSelectPage={onSelectPage}
            />
          </div>
        </div>
      </main>

      <TaskFiltersModal
        isOpen={filtersOpen}
        filters={collection.activeFilters}
        users={collection.lookups.users}
        clients={collection.lookups.clients}
        tracks={collection.lookups.tracks.map((track) => track.value)}
        onClose={onCloseFilters}
        onApply={onApplyFilters}
      />
      <AddTaskModal
        isOpen={addTaskOpen}
        trackSuggestions={collection.lookups.tracks.map((track) => track.value)}
        prefillAssignee={prefillAssignee}
        onClose={onCloseAddTask}
        onMutationSuccess={onMutationSuccess}
      />
    </TodoShell>
  );
}

export function TaskListPage() {
  const shellState = useTodoShell("Не удалось загрузить оболочку ToDo.");
  const [locationSearch, setLocationSearch] = useState(
    () => window.location.search,
  );
  const [collectionState, setCollectionState] = useState<CollectionLoadState>({
    status: "loading",
  });
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [addTaskOpen, setAddTaskOpen] = useState(false);
  const [refreshToken, setRefreshToken] = useState(0);
  const [prefillAssignee, setPrefillAssignee] = useState<
    TaskAssigneePrefill | undefined
  >(undefined);

  useEffect(() => {
    const handlePopState = () => {
      startTransition(() => {
        setLocationSearch(window.location.search);
      });
    };

    window.addEventListener("popstate", handlePopState);
    return () => {
      window.removeEventListener("popstate", handlePopState);
    };
  }, []);

  useEffect(() => {
    const assigneePrefill = readTaskAssigneePrefill(locationSearch);
    if (!assigneePrefill) {
      return;
    }

    setPrefillAssignee(assigneePrefill);
    setAddTaskOpen(true);

    const strippedSearch = stripTaskTransientParams(locationSearch);
    window.history.replaceState(
      {},
      document.title,
      `${window.location.pathname}${strippedSearch}`,
    );
    startTransition(() => {
      setLocationSearch(strippedSearch);
    });
  }, [locationSearch]);

  useEffect(() => {
    let active = true;

    setCollectionState((current) =>
      current.status === "ready"
        ? { status: "loading", previousData: current.data }
        : current.status === "error" && current.previousData
          ? { status: "loading", previousData: current.previousData }
          : { status: "loading" },
    );

    void fetchTaskCollection(
      buildTaskCollectionParams(parseTaskListQuery(locationSearch)),
    )
      .then((data) => {
        if (!active) {
          return;
        }

        setCollectionState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setCollectionState((current) => ({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить список задач.",
          previousData:
            current.status === "loading" ? current.previousData : undefined,
        }));
      });

    return () => {
      active = false;
    };
  }, [locationSearch, refreshToken]);

  useEffect(() => {
    if (
      collectionState.status === "error" &&
      collectionState.previousData != null
    ) {
      window.showFlashMessage?.(collectionState.message, "danger");
    }
  }, [collectionState]);

  if (shellState.status === "error") {
    return <TodoShellFatalState message={shellState.message} />;
  }

  if (
    shellState.status === "loading" ||
    (collectionState.status === "loading" &&
      collectionState.previousData == null)
  ) {
    return null;
  }

  const collection =
    collectionState.status === "ready"
      ? collectionState.data
      : collectionState.previousData;

  if (collectionState.status === "error" && !collection) {
    return <TodoShellFatalState message={collectionState.message} />;
  }

  if (!collection) {
    return null;
  }

  const navigateToSearch = (nextSearch: string) => {
    window.history.pushState(
      {},
      document.title,
      `${window.location.pathname}${nextSearch}`,
    );
    startTransition(() => {
      setLocationSearch(nextSearch);
    });
  };

  return (
    <TaskListScreen
      shell={shellState.shell}
      fetchedMenuItems={shellState.authMenuItems}
      collection={collection}
      isRefreshing={collectionState.status === "loading"}
      filtersOpen={filtersOpen}
      addTaskOpen={addTaskOpen}
      prefillAssignee={prefillAssignee}
      onSearchSubmit={(search) => {
        navigateToSearch(
          buildTaskListSearch({
            ...collection.activeFilters,
            search,
            page: undefined,
          }),
        );
      }}
      onOpenFilters={() => setFiltersOpen(true)}
      onCloseFilters={() => setFiltersOpen(false)}
      onApplyFilters={(filters) => {
        navigateToSearch(buildTaskListSearch({ ...filters }));
      }}
      onOpenAddTask={() => {
        setPrefillAssignee(undefined);
        setAddTaskOpen(true);
      }}
      onCloseAddTask={() => {
        setPrefillAssignee(undefined);
        setAddTaskOpen(false);
      }}
      onMutationSuccess={(message) => {
        window.showFlashMessage?.(message, "primary");
        setAddTaskOpen(false);
        setPrefillAssignee(undefined);
        setRefreshToken((current) => current + 1);
      }}
      onSelectPage={(page) => {
        navigateToSearch(
          buildTaskListSearch({ ...collection.activeFilters, page }),
        );
      }}
    />
  );
}
