import { describe, expect, it } from "vitest";

import {
  buildTaskListSearch,
  hasActiveTaskFilters,
  parseTaskListQuery,
  readTaskAssigneePrefill,
  stripTaskTransientParams,
} from "./taskListQuery";

describe("taskListQuery", () => {
  it("parses and serializes supported list filters", () => {
    const parsed = parseTaskListQuery(
      "?page=3&search=report&status=InProgress&track=Support&assignee=9&client=7&priority=High&updated_after=2024-02-01&updated_before=2024-02-10&public_id=task-5",
    );

    expect(parsed).toEqual({
      page: 3,
      search: "report",
      status: "InProgress",
      track: "Support",
      assigneeId: 9,
      clientId: 7,
      priority: "High",
      updatedAfter: "2024-02-01",
      updatedBefore: "2024-02-10",
      publicId: "task-5",
    });
    expect(
      buildTaskListSearch({
        page: 3,
        search: "report",
        status: "InProgress",
        track: "Support",
        assigneeId: 9,
        clientId: 7,
        priority: "High",
        updatedAfter: "2024-02-01",
        updatedBefore: "2024-02-10",
        publicId: "task-5",
      }),
    ).toBe(
      "?page=3&search=report&status=InProgress&track=Support&assignee=9&client=7&priority=High&updated_after=2024-02-01&updated_before=2024-02-10&public_id=task-5",
    );
  });

  it("reports active filters and strips transient assignee prefill params", () => {
    expect(
      hasActiveTaskFilters({
        status: "Pending",
      }),
    ).toBe(true);
    expect(hasActiveTaskFilters({})).toBe(false);
    expect(
      readTaskAssigneePrefill("?name=Anna%20Worker&email=anna%40example.com"),
    ).toEqual({
      name: "Anna Worker",
      email: "anna@example.com",
    });
    expect(
      stripTaskTransientParams(
        "?status=Pending&name=Anna%20Worker&email=anna%40example.com",
      ),
    ).toBe("?status=Pending");
  });
});
