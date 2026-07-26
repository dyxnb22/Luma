use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

pub const MAX_DATABASE_PORTALS: usize = 500;
pub const MAX_DATABASE_SCHEMA_OBJECTS: usize = 500;
pub const MAX_DATABASE_SCHEMA_BYTES: usize = 256 * 1024;

pub fn validate_postgres_metadata(
    host: &str,
    port: u16,
    database: &str,
    username: &str,
) -> Result<(), String> {
    if port == 0 {
        return Err("PostgreSQL port must be between 1 and 65535".into());
    }
    if host.is_empty()
        || host.len() > 255
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".:-_[]".contains(&byte))
    {
        return Err("PostgreSQL host must be a hostname or IP address, not a DSN".into());
    }
    for (field, value) in [("database", database), ("username", username)] {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        {
            return Err(format!(
                "PostgreSQL {field} contains unsupported characters"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatabasePortalTarget {
    Sqlite {
        path: PathBuf,
    },
    Postgres {
        host: String,
        port: u16,
        database: String,
        username: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabasePortal {
    pub id: i64,
    pub label: String,
    pub target: DatabasePortalTarget,
    pub environment: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewDatabasePortal {
    pub label: String,
    pub target: DatabasePortalTarget,
    pub environment: String,
    pub now: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DatabasePortalsRepoError {
    #[error("database portal not found")]
    NotFound,
    #[error("database portal changed since it was shown")]
    Conflict,
    #[error("database portal label already exists")]
    Duplicate,
    #[error("database portals capacity reached ({MAX_DATABASE_PORTALS})")]
    Capacity,
    #[error("database portals store: {0}")]
    Store(String),
}

pub trait DatabasePortalsRepository: Send + Sync {
    fn list(&self) -> Result<Vec<DatabasePortal>, DatabasePortalsRepoError>;
    fn get(&self, id: i64) -> Result<Option<DatabasePortal>, DatabasePortalsRepoError>;
    fn insert(
        &self,
        portal: &NewDatabasePortal,
    ) -> Result<DatabasePortal, DatabasePortalsRepoError>;
    fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), DatabasePortalsRepoError>;
    fn backup(&self) -> Result<PathBuf, DatabasePortalsRepoError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSchemaObject {
    pub kind: String,
    pub name: String,
    pub table_name: String,
    pub ddl: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseClientPlan {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DatabasePlatformError {
    #[error("database operation cancelled")]
    Cancelled,
    #[error("database client is not configured: {0}")]
    NotConfigured(String),
    #[error("database target was not found")]
    NotFound,
    #[error("database target is invalid: {0}")]
    Invalid(String),
    #[error("database operation unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait DatabasePlatformPort: Send + Sync {
    async fn canonicalize_sqlite(
        &self,
        path: &Path,
        cancel: CancellationToken,
    ) -> Result<PathBuf, DatabasePlatformError>;

    async fn sqlite_schema(
        &self,
        path: &Path,
        cancel: CancellationToken,
    ) -> Result<Vec<DatabaseSchemaObject>, DatabasePlatformError>;

    async fn client_plan(
        &self,
        target: &DatabasePortalTarget,
        cancel: CancellationToken,
    ) -> Result<DatabaseClientPlan, DatabasePlatformError>;
}

pub struct MemoryDatabasePortalsRepository {
    portals: Mutex<Vec<DatabasePortal>>,
    next_id: Mutex<i64>,
}

impl Default for MemoryDatabasePortalsRepository {
    fn default() -> Self {
        Self {
            portals: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl DatabasePortalsRepository for MemoryDatabasePortalsRepository {
    fn list(&self) -> Result<Vec<DatabasePortal>, DatabasePortalsRepoError> {
        let mut portals = self.portals.lock().expect("database portals lock").clone();
        portals.sort_by(|left, right| {
            left.label
                .to_lowercase()
                .cmp(&right.label.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(portals)
    }

    fn get(&self, id: i64) -> Result<Option<DatabasePortal>, DatabasePortalsRepoError> {
        Ok(self
            .portals
            .lock()
            .expect("database portals lock")
            .iter()
            .find(|portal| portal.id == id)
            .cloned())
    }

    fn insert(
        &self,
        portal: &NewDatabasePortal,
    ) -> Result<DatabasePortal, DatabasePortalsRepoError> {
        let mut portals = self.portals.lock().expect("database portals lock");
        if portals.len() >= MAX_DATABASE_PORTALS {
            return Err(DatabasePortalsRepoError::Capacity);
        }
        if portals
            .iter()
            .any(|current| current.label.eq_ignore_ascii_case(&portal.label))
        {
            return Err(DatabasePortalsRepoError::Duplicate);
        }
        let mut next_id = self.next_id.lock().expect("database portal id lock");
        let stored = DatabasePortal {
            id: *next_id,
            label: portal.label.clone(),
            target: portal.target.clone(),
            environment: portal.environment.clone(),
            created_at: portal.now.clone(),
            updated_at: portal.now.clone(),
        };
        *next_id += 1;
        portals.push(stored.clone());
        Ok(stored)
    }

    fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), DatabasePortalsRepoError> {
        let mut portals = self.portals.lock().expect("database portals lock");
        let index = portals
            .iter()
            .position(|portal| portal.id == id)
            .ok_or(DatabasePortalsRepoError::NotFound)?;
        if portals[index].updated_at != expected_updated_at {
            return Err(DatabasePortalsRepoError::Conflict);
        }
        portals.remove(index);
        Ok(())
    }

    fn backup(&self) -> Result<PathBuf, DatabasePortalsRepoError> {
        Ok(PathBuf::from("/fixture/database-portals-backup.sqlite"))
    }
}

pub struct FakeDatabasePlatform {
    pub canonical: Mutex<Result<PathBuf, DatabasePlatformError>>,
    pub schema: Mutex<Result<Vec<DatabaseSchemaObject>, DatabasePlatformError>>,
    pub sqlite_program: String,
    pub psql_program: String,
    pub calls: Mutex<Vec<String>>,
}

impl FakeDatabasePlatform {
    pub fn new(canonical: PathBuf) -> Self {
        Self {
            canonical: Mutex::new(Ok(canonical)),
            schema: Mutex::new(Ok(Vec::new())),
            sqlite_program: "/fixture/sqlite3".into(),
            psql_program: "/fixture/psql".into(),
            calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl DatabasePlatformPort for FakeDatabasePlatform {
    async fn canonicalize_sqlite(
        &self,
        path: &Path,
        cancel: CancellationToken,
    ) -> Result<PathBuf, DatabasePlatformError> {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        self.calls
            .lock()
            .expect("database platform calls lock")
            .push(format!("canonicalize:{}", path.display()));
        self.canonical
            .lock()
            .expect("database canonical lock")
            .clone()
    }

    async fn sqlite_schema(
        &self,
        path: &Path,
        cancel: CancellationToken,
    ) -> Result<Vec<DatabaseSchemaObject>, DatabasePlatformError> {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        self.calls
            .lock()
            .expect("database platform calls lock")
            .push(format!("schema:{}", path.display()));
        self.schema.lock().expect("database schema lock").clone()
    }

    async fn client_plan(
        &self,
        target: &DatabasePortalTarget,
        cancel: CancellationToken,
    ) -> Result<DatabaseClientPlan, DatabasePlatformError> {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        self.calls
            .lock()
            .expect("database platform calls lock")
            .push("client_plan".into());
        Ok(match target {
            DatabasePortalTarget::Sqlite { path } => DatabaseClientPlan {
                program: self.sqlite_program.clone(),
                args: vec![path.display().to_string()],
            },
            DatabasePortalTarget::Postgres {
                host,
                port,
                database,
                username,
            } => DatabaseClientPlan {
                program: self.psql_program.clone(),
                args: postgres_client_args(host, *port, database, username),
            },
        })
    }
}

pub fn postgres_client_args(host: &str, port: u16, database: &str, username: &str) -> Vec<String> {
    vec![
        "--host".into(),
        host.into(),
        "--port".into(),
        port.to_string(),
        "--username".into(),
        username.into(),
        "--dbname".into(),
        database.into(),
    ]
}
