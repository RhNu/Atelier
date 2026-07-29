use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::Poll;
use std::time::Duration;

use rusqlite::Connection;

use crate::error::{DatabaseError, DatabaseResult};
use crate::schema::initialize_or_validate_schema;

#[derive(Clone)]
pub struct DatabaseConnection {
    inner: Arc<Mutex<Connection>>,
    transaction_gate: Arc<TransactionGateState>,
}

struct TransactionGateState {
    locked: AtomicBool,
    waiters: Mutex<Vec<std::task::Waker>>,
}

impl std::fmt::Debug for DatabaseConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConnection").finish_non_exhaustive()
    }
}

impl DatabaseConnection {
    /// Opens a file-backed `SQLite` database and initializes or validates its schema.
    ///
    /// # Errors
    /// Returns an error when the database file cannot be opened or uses an unsupported schema.
    pub fn open(path: impl AsRef<Path>) -> DatabaseResult<Self> {
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| DatabaseError::new(error.to_string()))?;
        }
        let connection = Connection::open(path)?;
        let this = Self::from_connection(connection)?;
        this.initialize_or_validate_schema()?;
        Ok(this)
    }

    /// Opens an in-memory `SQLite` database and initializes its schema.
    ///
    /// # Errors
    /// Returns an error when schema initialization fails.
    pub fn open_memory() -> DatabaseResult<Self> {
        let this = Self::from_connection(Connection::open_in_memory()?)?;
        this.initialize_or_validate_schema()?;
        Ok(this)
    }

    fn from_connection(connection: Connection) -> DatabaseResult<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Keep transient writer contention bounded instead of immediately
        // surfacing SQLITE_BUSY from the synchronous adapter boundary.
        connection.busy_timeout(Duration::from_secs(5))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            transaction_gate: Arc::new(TransactionGateState {
                locked: AtomicBool::new(false),
                waiters: Mutex::new(Vec::new()),
            }),
        })
    }

    fn initialize_or_validate_schema(&self) -> DatabaseResult<()> {
        let mut connection = self.lock()?;
        initialize_or_validate_schema(&mut connection)
    }

    pub(crate) fn lock(&self) -> DatabaseResult<MutexGuard<'_, Connection>> {
        self.inner
            .lock()
            .map_err(|error| DatabaseError::new(error.to_string()))
    }

    pub(crate) async fn acquire_transaction_gate(&self) -> DatabaseResult<DatabaseTransactionGate> {
        let state = Arc::clone(&self.transaction_gate);
        let poll_state = Arc::clone(&state);
        std::future::poll_fn(move |context| {
            if poll_state
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Poll::Ready(Ok(()));
            }

            let mut waiters = match poll_state.waiters.lock() {
                Ok(waiters) => waiters,
                Err(error) => return Poll::Ready(Err(DatabaseError::new(error.to_string()))),
            };
            waiters.retain(|waker| !waker.will_wake(context.waker()));
            waiters.push(context.waker().clone());
            drop(waiters);

            if poll_state
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                if let Ok(mut waiters) = poll_state.waiters.lock() {
                    waiters.retain(|waker| !waker.will_wake(context.waker()));
                }
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        })
        .await?;

        Ok(DatabaseTransactionGate { state })
    }
}

pub struct DatabaseTransactionGate {
    state: Arc<TransactionGateState>,
}

impl Drop for DatabaseTransactionGate {
    fn drop(&mut self) {
        self.state.locked.store(false, Ordering::Release);
        let waiters = self
            .state
            .waiters
            .lock()
            .map(|mut waiters| std::mem::take(&mut *waiters))
            .unwrap_or_default();
        for waker in waiters {
            waker.wake();
        }
    }
}
