-- Capture task lifecycle metadata and assignments.
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'Pending',
    due_date DATE,
    assigned_to INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    CONSTRAINT task_status_check CHECK (status IN (
        'Pending',
        'InProgress',
        'Blocked',
        'Completed',
        'Archived'
    )),
    CONSTRAINT task_assignee_fk FOREIGN KEY (assigned_to)
        REFERENCES users (id)
        ON DELETE SET NULL
);

-- Track assignment history for auditing purposes.
CREATE TABLE task_assignments (
    id INTEGER PRIMARY KEY,
    task_id INTEGER NOT NULL,
    hub_id INTEGER NOT NULL,
    assignee_id INTEGER NOT NULL,
    assigned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT task_assignment_task_fk FOREIGN KEY (task_id)
        REFERENCES tasks (id)
        ON DELETE CASCADE,
    CONSTRAINT task_assignment_assignee_fk FOREIGN KEY (assignee_id)
        REFERENCES users (id)
        ON DELETE CASCADE
);

CREATE INDEX tasks_hub_idx
    ON tasks (hub_id);

CREATE INDEX tasks_assigned_to_idx
    ON tasks (assigned_to);

CREATE INDEX task_assignments_task_idx
    ON task_assignments (task_id);

CREATE INDEX task_assignments_assignee_idx
    ON task_assignments (assignee_id);
