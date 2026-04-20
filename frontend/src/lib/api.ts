import {
  browserLocation,
  fetchHubMenuItems as fetchSharedHubMenuItems,
  fetchJson as fetchSharedJson,
  fetchNoAccessData as fetchSharedNoAccessData,
  fetchShellData as fetchSharedShellData,
} from "@pushkind/frontend-shell/shellApi";
import {
  isRecord,
  readArray,
  readNumber,
  readNumberArray,
  readOptionalNumber,
  readOptionalString,
  readRecord,
  readString,
} from "@pushkind/frontend-shell/json";
import {
  isApiMutationError,
  postEmpty,
  postForm,
  postMultipartForm,
  toFieldErrorMap,
  type ApiFieldError,
  type ApiMutationError,
  type ApiMutationSuccess,
} from "@pushkind/frontend-shell/mutations";

export { browserLocation, isApiMutationError, toFieldErrorMap };
export type { ApiFieldError, ApiMutationError, ApiMutationSuccess };
import type {
  ClientLookupItem,
  NoAccessData,
  ShellData,
  TaskClientSummary,
  TaskCollectionData,
  TaskCollectionFilters,
  TaskDetailsData,
  TaskDetailsTask,
  TaskEventItem,
  TaskListItem,
  TaskPagination,
  TaskUserSummary,
  TrackLookupItem,
  UserLookupItem,
  UserMenuItem,
} from "./models";

function parseTaskUserSummary(payload: unknown): TaskUserSummary {
  if (!isRecord(payload)) {
    throw new Error("Invalid task user payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    email: readString(payload, "email"),
  };
}

function parseTaskClientSummary(payload: unknown): TaskClientSummary {
  if (!isRecord(payload)) {
    throw new Error("Invalid task client payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    publicId: readString(payload, "public_id"),
    url: readOptionalString(payload, "url"),
  };
}

function parseTaskListItem(payload: unknown): TaskListItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid task list item payload.");
  }

  return {
    id: readNumber(payload, "id"),
    publicId: readOptionalString(payload, "public_id"),
    title: readString(payload, "title"),
    description: readOptionalString(payload, "description"),
    track: readOptionalString(payload, "track"),
    priority: readString(payload, "priority"),
    status: readString(payload, "status"),
    dueDate: readOptionalString(payload, "due_date"),
    assignee: payload.assignee
      ? parseTaskUserSummary(payload.assignee)
      : undefined,
    client: payload.client ? parseTaskClientSummary(payload.client) : undefined,
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
    completedAt: readOptionalString(payload, "completed_at"),
  };
}

function parseTaskPagination(payload: unknown): TaskPagination {
  if (!isRecord(payload)) {
    throw new Error("Invalid pagination payload.");
  }

  return {
    page: readNumber(payload, "page"),
    totalPages: readNumber(payload, "total_pages"),
  };
}

function parseTaskCollectionFilters(payload: unknown): TaskCollectionFilters {
  if (!isRecord(payload)) {
    throw new Error("Invalid task filters payload.");
  }

  return {
    search: readOptionalString(payload, "search"),
    status: readOptionalString(payload, "status"),
    track: readOptionalString(payload, "track"),
    assigneeId: readOptionalNumber(payload, "assignee_id"),
    clientId: readOptionalNumber(payload, "client_id"),
    priority: readOptionalString(payload, "priority"),
    updatedAfter: readOptionalString(payload, "updated_after"),
    updatedBefore: readOptionalString(payload, "updated_before"),
    publicId: readOptionalString(payload, "public_id"),
  };
}

function parseUserLookupItems(payload: unknown): UserLookupItem[] {
  if (!isRecord(payload)) {
    throw new Error("Invalid users lookup payload.");
  }

  return readArray(payload, "items").map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid user lookup item payload.");
    }

    return {
      id: readNumber(item, "id"),
      name: readString(item, "name"),
      email: readString(item, "email"),
    };
  });
}

function parseClientLookupItems(payload: unknown): ClientLookupItem[] {
  if (!isRecord(payload)) {
    throw new Error("Invalid clients lookup payload.");
  }

  return readArray(payload, "items").map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid client lookup item payload.");
    }

    return {
      id: readNumber(item, "id"),
      name: readString(item, "name"),
      publicId: readString(item, "public_id"),
    };
  });
}

function parseTrackLookupItems(payload: unknown): TrackLookupItem[] {
  if (!isRecord(payload)) {
    throw new Error("Invalid tracks lookup payload.");
  }

  return readArray(payload, "items").map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid track lookup item payload.");
    }

    return {
      value: readString(item, "value"),
    };
  });
}

function parseTaskDetailsTask(payload: unknown): TaskDetailsTask {
  if (!isRecord(payload)) {
    throw new Error("Invalid task details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    publicId: readOptionalString(payload, "public_id"),
    title: readString(payload, "title"),
    description: readOptionalString(payload, "description"),
    track: readOptionalString(payload, "track"),
    priority: readString(payload, "priority"),
    status: readString(payload, "status"),
    dueDate: readOptionalString(payload, "due_date"),
    authorId: readNumber(payload, "author_id"),
    assigneeId: readOptionalNumber(payload, "assignee_id"),
    clientId: readOptionalNumber(payload, "client_id"),
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
    completedAt: readOptionalString(payload, "completed_at"),
  };
}

function parseTaskEventItem(payload: unknown): TaskEventItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid task event payload.");
  }

  return {
    id: readNumber(payload, "id"),
    eventType: readString(payload, "event_type"),
    eventData: payload.event_data,
    createdAt: readString(payload, "created_at"),
    author: payload.author ? parseTaskUserSummary(payload.author) : undefined,
  };
}

function parseTaskCollection(payload: unknown): TaskCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid task collection payload.");
  }

  const lookups = readRecord(payload, "lookups");

  return {
    items: readArray(payload, "items").map(parseTaskListItem),
    pagination: parseTaskPagination(payload.pagination),
    activeFilters: parseTaskCollectionFilters(payload.active_filters),
    recentlyUpdatedTaskIds: readNumberArray(
      payload,
      "recently_updated_task_ids",
    ),
    filesServiceUrl: readString(payload, "files_service_url"),
    lookups: {
      users: parseUserLookupItems(lookups.users),
      clients: parseClientLookupItems(lookups.clients),
      tracks: parseTrackLookupItems(lookups.tracks),
    },
  };
}

function parseTaskDetails(payload: unknown): TaskDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid task details response.");
  }

  return {
    task: parseTaskDetailsTask(payload.task),
    author: parseTaskUserSummary(payload.author),
    assignee: payload.assignee
      ? parseTaskUserSummary(payload.assignee)
      : undefined,
    client: payload.client ? parseTaskClientSummary(payload.client) : undefined,
    events: readArray(payload, "events").map(parseTaskEventItem),
    filesServiceUrl: readString(payload, "files_service_url"),
  };
}

export class ApiResponseFailure extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiResponseFailure";
    this.status = status;
  }
}

function withBaseUrl(baseUrl: string, path: string) {
  return new URL(path, baseUrl).toString();
}

function withQuery(path: string, params?: URLSearchParams) {
  if (!params || Array.from(params.keys()).length === 0) {
    return path;
  }

  return `${path}?${params.toString()}`;
}

async function fetchJson(url: string) {
  try {
    return await fetchSharedJson(url, {
      unauthorizedMessage: "Недостаточно прав для доступа к ToDo.",
      notFoundMessage: "Ресурс не найден.",
    });
  } catch (error) {
    if (error instanceof Error && error.message === "Ресурс не найден.") {
      throw new ApiResponseFailure(404, error.message);
    }

    throw error;
  }
}

export async function fetchShellData(): Promise<ShellData> {
  return fetchSharedShellData<ShellData>(
    "/api/v1/iam",
    "Недостаточно прав для доступа к ToDo.",
  );
}

export async function fetchNoAccessData(): Promise<NoAccessData> {
  const payload = await fetchSharedNoAccessData<NoAccessData>(
    "/api/v1/no-access",
    "Недостаточно прав для доступа к ToDo.",
  );
  return {
    ...payload,
    requiredRole: payload.requiredRole ?? "",
  };
}

export async function fetchTaskCollection(
  params?: URLSearchParams,
): Promise<TaskCollectionData> {
  const payload = await fetchJson(withQuery("/api/v1/tasks", params));
  return parseTaskCollection(payload);
}

export async function fetchTaskDetails(
  taskId: number,
): Promise<TaskDetailsData> {
  const payload = await fetchJson(`/api/v1/tasks/${taskId}`);
  return parseTaskDetails(payload);
}

export async function fetchUsers(query?: string): Promise<UserLookupItem[]> {
  const params = query ? new URLSearchParams({ query }) : undefined;
  const payload = await fetchJson(withQuery("/api/v1/users", params));
  return parseUserLookupItems(payload);
}

export async function fetchClients(
  search?: string,
): Promise<ClientLookupItem[]> {
  const params = search ? new URLSearchParams({ search }) : undefined;
  const payload = await fetchJson(withQuery("/api/v1/clients", params));
  return parseClientLookupItems(payload);
}

export async function fetchTracks(query?: string): Promise<TrackLookupItem[]> {
  const params = query ? new URLSearchParams({ query }) : undefined;
  const payload = await fetchJson(withQuery("/api/v1/tracks", params));
  return parseTrackLookupItems(payload);
}

export async function createTask(
  form: URLSearchParams,
): Promise<ApiMutationSuccess> {
  return postForm("/api/v1/tasks", form);
}

export async function uploadTasks(form: FormData): Promise<ApiMutationSuccess> {
  return postMultipartForm("/api/v1/tasks/upload", form);
}

export async function updateTask(
  taskId: number,
  form: URLSearchParams,
): Promise<ApiMutationSuccess> {
  return postForm(`/api/v1/tasks/${taskId}/update`, form);
}

export async function updateTaskStatus(
  taskId: number,
  form: URLSearchParams,
): Promise<ApiMutationSuccess> {
  return postForm(`/api/v1/tasks/${taskId}/status`, form);
}

export async function createTaskComment(
  taskId: number,
  form: URLSearchParams,
): Promise<ApiMutationSuccess> {
  return postForm(`/api/v1/tasks/${taskId}/comments`, form);
}

export async function deleteTask(taskId: number): Promise<ApiMutationSuccess> {
  return postEmpty(`/api/v1/tasks/${taskId}/delete`);
}

export async function fetchHubMenuItems(
  authBaseUrl: string,
  hubId: number,
): Promise<UserMenuItem[]> {
  return fetchSharedHubMenuItems<UserMenuItem>(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
    "Недостаточно прав для доступа к ToDo.",
  );
}
