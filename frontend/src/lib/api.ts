import type {
  ApiFieldError,
  ApiMutationError,
  ApiMutationSuccess,
  ClientLookupItem,
  CurrentUser,
  NavigationItem,
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readOptionalString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value === undefined || value === null) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string | null at ${key}.`);
  }

  return value;
}

function readNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number at ${key}.`);
  }

  return value;
}

function readOptionalNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value === undefined || value === null) {
    return undefined;
  }
  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number | null at ${key}.`);
  }

  return value;
}

function readStringArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`Invalid API response: expected string[] at ${key}.`);
  }

  return value;
}

function readNumberArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "number")) {
    throw new Error(`Invalid API response: expected number[] at ${key}.`);
  }

  return value;
}

function readArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`Invalid API response: expected array at ${key}.`);
  }

  return value;
}

function readRecord(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`Invalid API response: expected object at ${key}.`);
  }

  return value;
}

function parseNavigationItems(payload: unknown): NavigationItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid navigation payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid navigation item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseMenuItems(payload: unknown): UserMenuItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid menu item payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid menu item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseCurrentUser(payload: unknown): CurrentUser {
  if (!isRecord(payload)) {
    throw new Error("Invalid current user payload.");
  }

  return {
    email: readString(payload, "email"),
    name: readString(payload, "name"),
    hubId: readNumber(payload, "hub_id"),
    roles: readStringArray(payload, "roles"),
  };
}

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

function parseShellData(payload: unknown): ShellData {
  if (!isRecord(payload)) {
    throw new Error("Invalid shell payload.");
  }

  return {
    currentUser: parseCurrentUser(payload.current_user),
    homeUrl: readString(payload, "home_url"),
    navigation: parseNavigationItems(payload.navigation),
    localMenuItems: parseMenuItems(payload.local_menu_items),
  };
}

function parseNoAccessData(payload: unknown): NoAccessData {
  if (!isRecord(payload)) {
    throw new Error("Invalid no-access payload.");
  }

  return {
    currentUser: parseCurrentUser(payload.current_user),
    homeUrl: readString(payload, "home_url"),
    requiredRole: readString(payload, "required_role"),
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
  };
}

function parseApiFieldErrors(payload: unknown): ApiFieldError[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid mutation field errors payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid mutation field error payload.");
    }

    return {
      field: readString(item, "field"),
      message: readString(item, "message"),
    };
  });
}

export function parseApiMutationSuccess(payload: unknown): ApiMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid mutation success payload.");
  }

  return {
    message: readString(payload, "message"),
    redirectTo: readOptionalString(payload, "redirect_to"),
  };
}

export function parseApiMutationError(payload: unknown): ApiMutationError {
  if (!isRecord(payload)) {
    throw new Error("Invalid mutation error payload.");
  }

  return {
    message: readString(payload, "message"),
    fieldErrors: parseApiFieldErrors(payload.field_errors),
  };
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

function isJsonResponse(response: Response): boolean {
  return (
    response.headers.get("content-type")?.includes("application/json") ?? false
  );
}

export const browserLocation = {
  assign(url: string) {
    window.location.assign(url);
  },
};

function handleAuthRedirectResponse(response: Response): never {
  browserLocation.assign(response.url);
  throw new Error("Сессия истекла. Выполняется переход на страницу входа.");
}

function ensureResponseIsNotAuthRedirect(response: Response) {
  if (response.redirected && !isJsonResponse(response)) {
    handleAuthRedirectResponse(response);
  }
}

async function readJsonResponse<T>(response: Response, endpoint: string) {
  if (!isJsonResponse(response)) {
    throw new Error(
      `Expected JSON response from ${endpoint} with status ${response.status}.`,
    );
  }

  return (await response.json()) as T;
}

async function fetchJson(url: string) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
    },
    cache: "no-store",
    credentials: "include",
  });

  if (!response.ok) {
    if (response.status === 401) {
      throw new Error("Недостаточно прав для доступа к ToDo.");
    }

    throw new Error(`Request failed with status ${response.status}.`);
  }

  ensureResponseIsNotAuthRedirect(response);
  return readJsonResponse(response, url);
}

export async function fetchShellData(): Promise<ShellData> {
  const payload = await fetchJson("/api/v1/iam");
  return parseShellData(payload);
}

export async function fetchNoAccessData(): Promise<NoAccessData> {
  const payload = await fetchJson("/api/v1/no-access");
  return parseNoAccessData(payload);
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

export async function fetchHubMenuItems(
  authBaseUrl: string,
  hubId: number,
): Promise<UserMenuItem[]> {
  const payload = await fetchJson(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
  );
  return parseMenuItems(payload);
}
