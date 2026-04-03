import { afterEach, describe, expect, it, vi } from "vitest";

import {
  ApiMutationFailure,
  createTask,
  fetchClients,
  fetchTaskCollection,
  fetchTaskDetails,
  fetchTracks,
  fetchUsers,
  parseApiMutationError,
  parseApiMutationSuccess,
  uploadTasks,
} from "./api";

function jsonResponse(payload: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(payload), {
    status: 200,
    headers: {
      "content-type": "application/json",
    },
    ...init,
  });
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("api", () => {
  it("fetchTaskCollection parses collection payloads", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        items: [
          {
            id: 5,
            public_id: "task-5",
            title: "Prepare report",
            description: "<p>Body</p>",
            track: "Support",
            priority: "High",
            status: "InProgress",
            due_date: "2024-02-10",
            assignee: {
              id: 9,
              name: "Worker",
              email: "worker@example.com",
            },
            client: {
              id: 7,
              name: "ACME",
              public_id: "client-7",
            },
            created_at: "2024-02-01T09:30:00",
            updated_at: "2024-02-02T10:45:00",
            completed_at: null,
          },
        ],
        pagination: {
          page: 2,
          total_pages: 4,
        },
        active_filters: {
          search: "report",
          status: "InProgress",
          track: "Support",
          assignee_id: 9,
          client_id: 7,
          priority: "High",
          updated_after: "2024-02-01",
          updated_before: null,
          public_id: "task-5",
        },
        recently_updated_task_ids: [5],
        lookups: {
          users: {
            items: [{ id: 9, name: "Worker", email: "worker@example.com" }],
          },
          clients: {
            items: [{ id: 7, name: "ACME", public_id: "client-7" }],
          },
          tracks: {
            items: [{ value: "Support" }],
          },
        },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const response = await fetchTaskCollection(
      new URLSearchParams({ status: "InProgress" }),
    );

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/tasks?status=InProgress",
      expect.objectContaining({
        credentials: "include",
      }),
    );
    expect(response.items[0].client?.publicId).toBe("client-7");
    expect(response.pagination.totalPages).toBe(4);
    expect(response.activeFilters.assigneeId).toBe(9);
    expect(response.lookups.tracks[0].value).toBe("Support");
  });

  it("fetchTaskDetails parses nested task timeline payloads", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        jsonResponse({
          task: {
            id: 5,
            public_id: "task-5",
            title: "Prepare report",
            description: "<p>Body</p>",
            track: "Support",
            priority: "High",
            status: "InProgress",
            due_date: "2024-02-10",
            author_id: 1,
            assignee_id: 9,
            client_id: 7,
            created_at: "2024-02-01T09:30:00",
            updated_at: "2024-02-02T10:45:00",
            completed_at: null,
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
            public_id: "client-7",
          },
          events: [
            {
              id: 11,
              event_type: "Comment",
              event_data: { text: "Started" },
              created_at: "2024-02-02T12:00:00",
              author: {
                id: 1,
                name: "Author",
                email: "author@example.com",
              },
            },
          ],
        }),
      ),
    );

    const response = await fetchTaskDetails(5);

    expect(response.task.authorId).toBe(1);
    expect(response.assignee?.email).toBe("worker@example.com");
    expect(response.events[0].eventType).toBe("Comment");
    expect(response.events[0].eventData).toEqual({ text: "Started" });
  });

  it("lookup fetchers parse collection contracts", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          items: [{ id: 9, name: "Worker", email: "worker@example.com" }],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          items: [{ id: 7, name: "ACME", public_id: "client-7" }],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          items: [{ value: "Support" }],
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const users = await fetchUsers("wor");
    const clients = await fetchClients("ac");
    const tracks = await fetchTracks("sup");

    expect(users[0].email).toBe("worker@example.com");
    expect(clients[0].publicId).toBe("client-7");
    expect(tracks[0].value).toBe("Support");
  });

  it("mutation parsers map snake_case envelopes to frontend models", () => {
    const success = parseApiMutationSuccess({
      message: "Saved",
      redirect_to: "/task/5",
    });
    const error = parseApiMutationError({
      message: "Validation failed",
      field_errors: [{ field: "title", message: "Required" }],
    });

    expect(success.redirectTo).toBe("/task/5");
    expect(error.fieldErrors).toEqual([
      { field: "title", message: "Required" },
    ]);
  });

  it("createTask submits urlencoded data and parses success envelopes", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        message: "Задача добавлена.",
        redirect_to: null,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const response = await createTask(
      new URLSearchParams({
        title: "Prepare report",
        priority: "High",
      }),
    );

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/tasks",
      expect.objectContaining({
        method: "POST",
        credentials: "include",
      }),
    );
    expect(response.message).toBe("Задача добавлена.");
  });

  it("uploadTasks parses structured mutation failures", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        jsonResponse(
          {
            message: "Ошибка валидации формы.",
            field_errors: [
              { field: "csv", message: "Не удалось обработать CSV-файл." },
            ],
          },
          { status: 400 },
        ),
      ),
    );

    const form = new FormData();
    form.set("csv", new File(["bad"], "tasks.csv", { type: "text/csv" }));

    const error = await uploadTasks(form).catch((caughtError) => caughtError);

    expect(error).toBeInstanceOf(ApiMutationFailure);
    expect(error).toMatchObject({
      status: 400,
      payload: {
        fieldErrors: [
          { field: "csv", message: "Не удалось обработать CSV-файл." },
        ],
      },
    });
  });

  it("surfaces unauthorized responses with a localized error", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(null, {
          status: 401,
          headers: { "content-type": "application/json" },
        }),
      ),
    );

    await expect(fetchTaskCollection()).rejects.toThrow(
      "Недостаточно прав для доступа к ToDo.",
    );
  });
});
