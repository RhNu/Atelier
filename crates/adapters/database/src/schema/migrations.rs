use rusqlite::Connection;

use crate::error::{DatabaseError, DatabaseResult};

mod v1_to_v2;
mod v2_to_v3;
mod v3_to_v4;
mod v4_to_v5;
mod v5_to_v6;
mod v6_to_v7;
mod v7_to_v8;

pub(super) fn migrate(
    connection: &mut Connection,
    mut version: i64,
    target: i64,
) -> DatabaseResult<()> {
    while version < target {
        version = match version {
            1 => v1_to_v2::migrate(connection)?,
            2 => v2_to_v3::migrate(connection)?,
            3 => v3_to_v4::migrate(connection)?,
            4 => v4_to_v5::migrate(connection)?,
            5 => v5_to_v6::migrate(connection)?,
            6 => v6_to_v7::migrate(connection)?,
            7 => v7_to_v8::migrate(connection)?,
            unsupported => {
                return Err(DatabaseError::unsupported_schema(format!(
                    "no database migration starts at schema version {unsupported}"
                )));
            }
        };
    }
    Ok(())
}
