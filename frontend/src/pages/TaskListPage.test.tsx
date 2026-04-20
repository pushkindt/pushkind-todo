import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { TaskListScreen } from "./TaskListPage";

const shell = {
  currentUser: {
    email: "user@example.com",
    name: "User",
    hubId: 1,
    roles: ["todo"],
  },
  homeUrl: "https://auth.example.com",
  navigation: [{ name: "Задачи", url: "/" }],
  localMenuItems: [],
};

describe("TaskListScreen", () => {
  it("renders task rows, active filter badge, and main-branch navbar search markup", () => {
    const markup = renderToStaticMarkup(
      <TaskListScreen
        shell={shell}
        fetchedMenuItems={[]}
        collection={{
          items: [
            {
              id: 5,
              title: "Prepare report",
              description: "<p>Body</p>",
              track: "Support",
              priority: "High",
              status: "InProgress",
              createdAt: "2024-02-01T09:30:00",
              updatedAt: "2024-02-02T10:45:00",
            },
          ],
          pagination: {
            page: 2,
            totalPages: 4,
          },
          activeFilters: {
            search: "report",
            status: "InProgress",
          },
          recentlyUpdatedTaskIds: [5],
          lookups: {
            users: [],
            clients: [],
            tracks: [{ value: "Support" }],
          },
          filesServiceUrl: "https://files.example.com",
        }}
        isRefreshing={false}
        filtersOpen={false}
        addTaskOpen={false}
        onSearchSubmit={() => {}}
        onOpenFilters={() => {}}
        onCloseFilters={() => {}}
        onApplyFilters={() => {}}
        onOpenAddTask={() => {}}
        onCloseAddTask={() => {}}
        onMutationSuccess={() => {}}
        onSelectPage={() => {}}
      />,
    );

    expect(markup).toContain("Prepare report");
    expect(markup).toContain("В работе");
    expect(markup).toContain("Высокий");
    expect(markup).toContain("task-recent");
    expect(markup).toContain('placeholder="Поиск"');
    expect(markup).toContain('value="report"');
    expect(markup).toContain("input-group");
    expect(markup).toContain("bi-search");
  });

  it("renders the empty state when no tasks are present", () => {
    const markup = renderToStaticMarkup(
      <TaskListScreen
        shell={shell}
        fetchedMenuItems={[]}
        collection={{
          items: [],
          pagination: {
            page: 1,
            totalPages: 1,
          },
          activeFilters: {},
          recentlyUpdatedTaskIds: [],
          lookups: {
            users: [],
            clients: [],
            tracks: [],
          },
          filesServiceUrl: "https://files.example.com",
        }}
        isRefreshing={false}
        filtersOpen={false}
        addTaskOpen={false}
        onSearchSubmit={() => {}}
        onOpenFilters={() => {}}
        onCloseFilters={() => {}}
        onApplyFilters={() => {}}
        onOpenAddTask={() => {}}
        onCloseAddTask={() => {}}
        onMutationSuccess={() => {}}
        onSelectPage={() => {}}
      />,
    );

    expect(markup).toContain("Нет задач для отображения.");
  });
});
