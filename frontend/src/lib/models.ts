import type {
  FrontendNoAccessData,
  FrontendShellCurrentUser,
  FrontendShellData,
  FrontendShellNavigationItem,
  FrontendShellUserMenuItem,
} from "@pushkind/frontend-shell/types";
import type {
  ApiFieldError as SharedApiFieldError,
  ApiMutationError as SharedApiMutationError,
  ApiMutationSuccess as SharedApiMutationSuccess,
} from "@pushkind/frontend-shell/mutations";

export type NavigationItem = FrontendShellNavigationItem;
export type UserMenuItem = FrontendShellUserMenuItem;
export type CurrentUser = FrontendShellCurrentUser;
export type ShellData = FrontendShellData;
export type NoAccessData = Omit<
  FrontendNoAccessData<CurrentUser>,
  "requiredRole"
> & {
  requiredRole: string;
};

export type TaskUserSummary = {
  id: number;
  name: string;
  email: string;
};

export type TaskClientSummary = {
  id: number;
  name: string;
  publicId: string;
  url?: string;
};

export type TaskListItem = {
  id: number;
  publicId?: string;
  title: string;
  description?: string;
  track?: string;
  priority: string;
  status: string;
  dueDate?: string;
  assignee?: TaskUserSummary;
  client?: TaskClientSummary;
  createdAt: string;
  updatedAt: string;
  completedAt?: string;
};

export type TaskPagination = {
  page: number;
  totalPages: number;
};

export type TaskCollectionFilters = {
  search?: string;
  status?: string;
  track?: string;
  assigneeId?: number;
  clientId?: number;
  priority?: string;
  updatedAfter?: string;
  updatedBefore?: string;
  publicId?: string;
};

export type UserLookupItem = {
  id: number;
  name: string;
  email: string;
};

export type ClientLookupItem = {
  id: number;
  name: string;
  publicId: string;
};

export type TrackLookupItem = {
  value: string;
};

export type TaskCollectionLookups = {
  users: UserLookupItem[];
  clients: ClientLookupItem[];
  tracks: TrackLookupItem[];
};

export type TaskCollectionData = {
  items: TaskListItem[];
  pagination: TaskPagination;
  activeFilters: TaskCollectionFilters;
  recentlyUpdatedTaskIds: number[];
  lookups: TaskCollectionLookups;
  filesServiceUrl: string;
};

export type TaskDetailsTask = {
  id: number;
  publicId?: string;
  title: string;
  description?: string;
  track?: string;
  priority: string;
  status: string;
  dueDate?: string;
  authorId: number;
  assigneeId?: number;
  clientId?: number;
  createdAt: string;
  updatedAt: string;
  completedAt?: string;
};

export type TaskEventItem = {
  id: number;
  eventType: string;
  eventData: unknown;
  createdAt: string;
  author?: TaskUserSummary;
};

export type TaskDetailsData = {
  task: TaskDetailsTask;
  author: TaskUserSummary;
  assignee?: TaskUserSummary;
  client?: TaskClientSummary;
  events: TaskEventItem[];
  filesServiceUrl: string;
};

export type ApiFieldError = SharedApiFieldError;
export type ApiMutationSuccess = SharedApiMutationSuccess;
export type ApiMutationError = SharedApiMutationError;
