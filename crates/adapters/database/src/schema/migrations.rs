use rusqlite::Connection;

use crate::error::{DatabaseError, DatabaseResult};

mod v1_to_v2;

pub(super) fn migrate(
    connection: &mut Connection,
    mut version: i64,
    target: i64,
) -> DatabaseResult<()> {
    while version < target {
        version = match version {
            1 => v1_to_v2::migrate(connection)?,
            unsupported => {
                return Err(DatabaseError::unsupported_schema(format!(
                    "no database migration starts at schema version {unsupported}"
                )));
            }
        };
    }
    Ok(())
}
