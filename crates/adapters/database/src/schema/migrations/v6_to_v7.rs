use crate::error::DatabaseResult;

const TO_VERSION: i64 = 7;

pub(super) fn migrate(connection: &mut rusqlite::Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        r"
        CREATE TABLE agent_sessions (
            session_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('idle', 'running', 'interrupted')),
            model_json TEXT NOT NULL,
            persona_json TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX idx_agent_sessions_updated_at
            ON agent_sessions(updated_at_ms DESC, session_id ASC);
        CREATE TABLE agent_events (
            event_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            event_kind TEXT NOT NULL,
            event_json TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id) ON DELETE CASCADE,
            UNIQUE (session_id, sequence)
        );
        CREATE INDEX idx_agent_events_session_sequence
            ON agent_events(session_id, sequence);
        CREATE TABLE agent_actions (
            action_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            base_revision INTEGER NOT NULL,
            applied_revision INTEGER NOT NULL,
            before_json TEXT NOT NULL,
            after_json TEXT NOT NULL,
            action_state TEXT NOT NULL CHECK (action_state IN ('applied', 'undone')),
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id) ON DELETE CASCADE
        );
        CREATE TABLE agent_summaries (
            session_id TEXT PRIMARY KEY,
            through_sequence INTEGER NOT NULL,
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(session_id) ON DELETE CASCADE
        );
        ",
    )?;
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = ?1 WHERE singleton = 1",
        [TO_VERSION],
    )?;
    transaction.commit()?;
    Ok(TO_VERSION)
}
