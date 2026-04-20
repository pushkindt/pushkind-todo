import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { TaskDetailsScreen } from "./TaskDetailsPage";

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

function renderScreen(
  overrides?: Partial<Parameters<typeof TaskDetailsScreen>[0]>,
) {
  return renderToStaticMarkup(
    <TaskDetailsScreen
      shell={shell}
      fetchedMenuItems={[]}
      details={{
        task: {
          id: 5,
          publicId: "task-5",
          title: "Prepare report",
          description: "<p>Body</p>",
          track: "Support",
          priority: "High",
          status: "Pending",
          dueDate: "2024-02-10",
          authorId: 1,
          assigneeId: 9,
          clientId: 7,
          createdAt: "2024-02-01T09:30:00",
          updatedAt: "2024-02-02T10:45:00",
        },
        author: {
          id: 1,
          name: "Author",
          email: "author@example.com",
        },
        assignee: {
          id: 9,
          name: "Worker",
          email: "worker@example.com",
        },
        client: {
          id: 7,
          name: "ACME",
          publicId: "client-7",
          url: "https://crm.example.com/?public_id=client-7",
        },
        events: [
          {
            id: 11,
            eventType: "Comment",
            eventData: { text: "<p>Принято в работу</p>" },
            createdAt: "2024-02-02T12:00:00",
            author: {
              id: 1,
              name: "Author",
              email: "author@example.com",
            },
          },
        ],
        filesServiceUrl: "https://files.example.com",
      }}
      filesServiceUrl="https://files.example.com"
      isRefreshing={false}
      editOpen={false}
      completeOpen={false}
      deleteOpen={false}
      quickActionSubmitting={false}
      commentMarkdown=""
      commentErrorMessage=""
      commentSubmitting={false}
      completeComment=""
      completeErrorMessage=""
      completeSubmitting={false}
      deleteSubmitting={false}
      onOpenEdit={() => {}}
      onCloseEdit={() => {}}
      onRequestDelete={() => {}}
      onCloseDelete={() => {}}
      onConfirmDelete={() => {}}
      onTakeInWork={() => {}}
      onOpenComplete={() => {}}
      onCloseComplete={() => {}}
      onCommentChange={() => {}}
      onSubmitComment={() => {}}
      onCompleteCommentChange={() => {}}
      onSubmitComplete={() => {}}
      onMutationSuccess={() => {}}
      {...overrides}
    />,
  );
}

describe("TaskDetailsScreen", () => {
  it("renders task metadata, the legacy markdown composer structure, and the event timeline", () => {
    const markup = renderScreen();

    expect(markup).toContain("Prepare report");
    expect(markup).toContain("Взять в работу");
    expect(markup).toContain(
      'href="https://crm.example.com/?public_id=client-7"',
    );
    expect(markup).toContain("Принято в работу");
    expect(markup).toContain("История событий");
    expect(markup).toContain("shell-markdown-composer");
    expect(markup).toContain("Файлы");
    expect(markup).toContain("border-top-0 rounded-top-0");
    expect(markup).toContain('data-bs-toggle="popover"');
  });

  it("renders the empty timeline state and hides quick actions for completed tasks", () => {
    const markup = renderScreen({
      details: {
        task: {
          id: 5,
          publicId: "task-5",
          title: "Prepare report",
          description: "<p>Body</p>",
          track: "Support",
          priority: "High",
          status: "Completed",
          dueDate: "2024-02-10",
          authorId: 1,
          assigneeId: 9,
          clientId: 7,
          createdAt: "2024-02-01T09:30:00",
          updatedAt: "2024-02-02T10:45:00",
        },
        author: {
          id: 1,
          name: "Author",
          email: "author@example.com",
        },
        assignee: {
          id: 9,
          name: "Worker",
          email: "worker@example.com",
        },
        client: {
          id: 7,
          name: "ACME",
          publicId: "client-7",
          url: "https://crm.example.com/?public_id=client-7",
        },
        events: [],
        filesServiceUrl: "https://files.example.com",
      },
    });

    expect(markup).toContain("Для этой задачи пока нет событий.");
    expect(markup).not.toContain("Взять в работу");
    expect(markup).not.toContain("Отметить как сделано");
  });
});
