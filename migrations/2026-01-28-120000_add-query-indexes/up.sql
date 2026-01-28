-- Add indexes to support common query filters and ordering.

CREATE INDEX users_hub_name_idx
    ON users (hub_id, name);

CREATE INDEX clients_hub_name_idx
    ON clients (hub_id, name);

CREATE INDEX tasks_hub_updated_at_idx
    ON tasks (hub_id, updated_at);

CREATE INDEX tasks_hub_assigned_to_idx
    ON tasks (hub_id, assigned_to);

CREATE INDEX tasks_hub_client_id_idx
    ON tasks (hub_id, client_id);

CREATE INDEX tasks_hub_status_idx
    ON tasks (hub_id, status);

CREATE INDEX tasks_hub_track_idx
    ON tasks (hub_id, track);

CREATE INDEX tasks_hub_priority_idx
    ON tasks (hub_id, priority);

CREATE INDEX tasks_hub_due_date_idx
    ON tasks (hub_id, due_date);

CREATE INDEX task_assignments_task_hub_assigned_at_idx
    ON task_assignments (task_id, hub_id, assigned_at);

CREATE INDEX task_assignments_task_hub_assignee_idx
    ON task_assignments (task_id, hub_id, assignee_id);

CREATE INDEX task_events_task_created_idx
    ON task_events (task_id, created_at);
