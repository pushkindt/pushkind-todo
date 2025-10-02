-- Clean up indexes and tables related to tasks.
DROP INDEX IF EXISTS task_assignments_assignee_idx;
DROP INDEX IF EXISTS task_assignments_task_idx;
DROP INDEX IF EXISTS tasks_assigned_to_idx;
DROP INDEX IF EXISTS tasks_hub_idx;

DROP TABLE IF EXISTS task_assignments;
DROP TABLE IF EXISTS tasks;
