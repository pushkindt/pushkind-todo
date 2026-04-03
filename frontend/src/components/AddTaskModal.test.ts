import { describe, expect, it } from "vitest";

import { unmatchedAssigneeError } from "./AddTaskModal";

describe("unmatchedAssigneeError", () => {
  it("rejects typed assignee text when no exact option is selected", () => {
    expect(unmatchedAssigneeError("User (user@example.com)", undefined)).toBe(
      "Выберите исполнителя из списка.",
    );
  });

  it("allows blank assignee input", () => {
    expect(unmatchedAssigneeError("   ", undefined)).toBeUndefined();
  });

  it("allows exact selected assignee values", () => {
    expect(
      unmatchedAssigneeError("User (user@example.com)", {
        name: "User",
        email: "user@example.com",
      }),
    ).toBeUndefined();
  });
});
