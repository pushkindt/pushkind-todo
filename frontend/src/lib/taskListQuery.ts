import type { TaskCollectionFilters } from "./models";

export type TaskListQueryState = TaskCollectionFilters & {
  page?: number;
};

export type TaskAssigneePrefill = {
  name: string;
  email: string;
};

function readTrimmed(params: URLSearchParams, key: string) {
  const value = params.get(key)?.trim();
  return value ? value : undefined;
}

function readPositiveNumber(params: URLSearchParams, key: string) {
  const value = params.get(key);
  if (!value) {
    return undefined;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
}

function createParams(search: string) {
  return new URLSearchParams(search.startsWith("?") ? search.slice(1) : search);
}

export function parseTaskListQuery(search: string): TaskListQueryState {
  const params = createParams(search);

  return {
    page: readPositiveNumber(params, "page"),
    search: readTrimmed(params, "search"),
    status: readTrimmed(params, "status"),
    track: readTrimmed(params, "track"),
    assigneeId: readPositiveNumber(params, "assignee"),
    clientId: readPositiveNumber(params, "client"),
    priority: readTrimmed(params, "priority"),
    updatedAfter: readTrimmed(params, "updated_after"),
    updatedBefore: readTrimmed(params, "updated_before"),
    publicId: readTrimmed(params, "public_id"),
  };
}

export function buildTaskListSearch(query: TaskListQueryState): string {
  const params = new URLSearchParams();

  if (query.page && query.page > 1) {
    params.set("page", query.page.toString());
  }
  if (query.search) {
    params.set("search", query.search);
  }
  if (query.status) {
    params.set("status", query.status);
  }
  if (query.track) {
    params.set("track", query.track);
  }
  if (query.assigneeId) {
    params.set("assignee", query.assigneeId.toString());
  }
  if (query.clientId) {
    params.set("client", query.clientId.toString());
  }
  if (query.priority) {
    params.set("priority", query.priority);
  }
  if (query.updatedAfter) {
    params.set("updated_after", query.updatedAfter);
  }
  if (query.updatedBefore) {
    params.set("updated_before", query.updatedBefore);
  }
  if (query.publicId) {
    params.set("public_id", query.publicId);
  }

  const serialized = params.toString();
  return serialized ? `?${serialized}` : "";
}

export function buildTaskCollectionParams(query: TaskListQueryState) {
  const search = buildTaskListSearch(query);
  return new URLSearchParams(search.startsWith("?") ? search.slice(1) : search);
}

export function hasActiveTaskFilters(query: TaskListQueryState) {
  return Boolean(
    query.search ||
    query.status ||
    query.track ||
    query.assigneeId ||
    query.clientId ||
    query.priority ||
    query.updatedAfter ||
    query.updatedBefore ||
    query.publicId,
  );
}

export function readTaskAssigneePrefill(
  search: string,
): TaskAssigneePrefill | undefined {
  const params = createParams(search);
  const name = readTrimmed(params, "name");
  const email = readTrimmed(params, "email");

  if (!name || !email) {
    return undefined;
  }

  return { name, email };
}

export function stripTaskTransientParams(search: string) {
  const params = createParams(search);
  params.delete("name");
  params.delete("email");

  const serialized = params.toString();
  return serialized ? `?${serialized}` : "";
}
