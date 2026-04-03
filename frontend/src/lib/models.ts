export type NavigationItem = {
  name: string;
  url: string;
};

export type UserMenuItem = {
  name: string;
  url: string;
  iconClass?: string;
};

export type CurrentUser = {
  email: string;
  name: string;
  hubId: number;
  roles: string[];
};

export type ShellData = {
  currentUser: CurrentUser;
  homeUrl: string;
  navigation: NavigationItem[];
  localMenuItems: UserMenuItem[];
};

export type NoAccessData = {
  currentUser: CurrentUser;
  homeUrl: string;
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
};

export type ApiFieldError = {
  field: string;
  message: string;
};

export type ApiMutationSuccess = {
  message: string;
  redirectTo?: string;
};

export type ApiMutationError = {
  message: string;
  fieldErrors: ApiFieldError[];
};
