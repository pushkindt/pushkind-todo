-- Record task activity to support auditing and notifications.
CREATE TABLE task_events (
    id INTEGER NOT NULL PRIMARY KEY,
    task_id INTEGER NOT NULL,
    user_id INTEGER,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT task_event_type_check CHECK (
        event_type IN (
            'Comment',
            'StatusChanged',
            'AssignmentChanged',
            'MetadataUpdated'
        )
    ),
    CONSTRAINT task_event_task_fk FOREIGN KEY (task_id)
        REFERENCES tasks (id)
        ON DELETE CASCADE,
    CONSTRAINT task_event_user_fk FOREIGN KEY (user_id)
        REFERENCES users (id)
        ON DELETE SET NULL
);

CREATE INDEX task_events_task_idx
    ON task_events (task_id);

CREATE INDEX task_events_user_idx
    ON task_events (user_id);

CREATE INDEX task_events_created_idx
    ON task_events (created_at);
