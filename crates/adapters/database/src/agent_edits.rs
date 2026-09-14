use atelier_agent::{AgentAction, AgentActionState, AgentError, AgentResult};
use atelier_generation::{GenerationDraftSnapshot, VersionedGenerationDraft};

use crate::DatabaseConnection;
use crate::agent::save_action_on_connection;
use crate::generation_draft::save_on_connection;

/// Commits a text-only draft edit and its undo record in the same SQLite transaction.
#[derive(Clone, Debug)]
pub struct DatabaseAgentEditStore {
    connection: DatabaseConnection,
}

impl DatabaseAgentEditStore {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    /// # Errors
    /// Rejects a stale draft, invalid snapshot, or failed action write without committing either.
    pub fn commit_draft(
        &self,
        expected_revision: u64,
        draft: &GenerationDraftSnapshot,
        mut action: AgentAction,
    ) -> AgentResult<VersionedGenerationDraft> {
        let mut connection = self.connection.lock().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        if action.state == AgentActionState::Undone {
            let state: String = transaction
                .query_row(
                    "SELECT action_state FROM agent_actions WHERE action_id = ?1",
                    [action.id.as_str()],
                    |row| row.get(0),
                )
                .map_err(repository_error)?;
            if state != "applied" {
                return Err(AgentError::conflict("action was already undone"));
            }
        }
        let saved = save_on_connection(&transaction, expected_revision, draft).map_err(
            |error| match error.kind {
                atelier_generation::GenerationDraftErrorKind::Conflict => {
                    AgentError::conflict(error.to_string())
                }
                _ => AgentError::repository(error.to_string()),
            },
        )?;
        if action.state == AgentActionState::Applied {
            action.applied_revision = saved.revision;
        }
        save_action_on_connection(&transaction, &action)?;
        transaction.commit().map_err(repository_error)?;
        drop(connection);
        Ok(saved)
    }
}

fn repository_error(error: impl std::fmt::Display) -> AgentError {
    AgentError::repository(error.to_string())
}
