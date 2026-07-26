use async_trait::async_trait;
use luma_application::{
    postgres_client_args, validate_postgres_metadata, DatabaseClientPlan, DatabasePlatformError,
    DatabasePlatformPort, DatabasePortalTarget, DatabaseSchemaObject, MAX_DATABASE_SCHEMA_BYTES,
    MAX_DATABASE_SCHEMA_OBJECTS,
};
use rusqlite::{Connection, OpenFlags};
use std::path::{Component, Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub struct MacDatabasePlatform;

#[async_trait]
impl DatabasePlatformPort for MacDatabasePlatform {
    async fn canonicalize_sqlite(
        &self,
        path: &Path,
        cancel: CancellationToken,
    ) -> Result<PathBuf, DatabasePlatformError> {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        let path = path.to_path_buf();
        let result = tokio::task::spawn_blocking(move || canonicalize_sqlite_sync(&path))
            .await
            .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        result
    }

    async fn sqlite_schema(
        &self,
        path: &Path,
        cancel: CancellationToken,
    ) -> Result<Vec<DatabaseSchemaObject>, DatabasePlatformError> {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        let path = path.to_path_buf();
        let worker_cancel = cancel.clone();
        let result =
            tokio::task::spawn_blocking(move || read_sqlite_schema_sync(&path, &worker_cancel))
                .await
                .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        result
    }

    async fn client_plan(
        &self,
        target: &DatabasePortalTarget,
        cancel: CancellationToken,
    ) -> Result<DatabaseClientPlan, DatabasePlatformError> {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        let plan = match target {
            DatabasePortalTarget::Sqlite { path } => {
                let canonical = self.canonicalize_sqlite(path, cancel.clone()).await?;
                let program = require_program("/usr/bin/sqlite3", "sqlite3")?;
                DatabaseClientPlan {
                    program,
                    args: vec![path_to_string(&canonical)?],
                }
            }
            DatabasePortalTarget::Postgres {
                host,
                port,
                database,
                username,
            } => {
                validate_postgres_metadata(host, *port, database, username)
                    .map_err(DatabasePlatformError::Invalid)?;
                let program = find_path_program("psql").ok_or_else(|| {
                    DatabasePlatformError::NotConfigured(
                        "psql is not executable on the current PATH".into(),
                    )
                })?;
                DatabaseClientPlan {
                    program: path_to_string(&program)?,
                    args: postgres_client_args(host, *port, database, username),
                }
            }
        };
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        Ok(plan)
    }
}

fn canonicalize_sqlite_sync(path: &Path) -> Result<PathBuf, DatabasePlatformError> {
    if path.as_os_str().is_empty() || path.components().any(|part| part == Component::ParentDir) {
        return Err(DatabasePlatformError::Invalid(
            "SQLite path must be explicit and cannot contain `..`".into(),
        ));
    }
    if path
        .to_str()
        .is_some_and(|value| value.chars().any(char::is_control))
    {
        return Err(DatabasePlatformError::Invalid(
            "SQLite path cannot contain control characters".into(),
        ));
    }
    let canonical = std::fs::canonicalize(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => DatabasePlatformError::NotFound,
        _ => DatabasePlatformError::Unavailable(error.to_string()),
    })?;
    let metadata = std::fs::metadata(&canonical)
        .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
    if !metadata.is_file() {
        return Err(DatabasePlatformError::Invalid(
            "SQLite target must be an existing regular file".into(),
        ));
    }
    if canonical.to_str().is_none() {
        return Err(DatabasePlatformError::Invalid(
            "SQLite path must be valid UTF-8 for the interactive terminal".into(),
        ));
    }
    Ok(canonical)
}

fn read_sqlite_schema_sync(
    path: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<DatabaseSchemaObject>, DatabasePlatformError> {
    let canonical = canonicalize_sqlite_sync(path)?;
    if cancel.is_cancelled() {
        return Err(DatabasePlatformError::Cancelled);
    }
    let connection = Connection::open_with_flags(
        canonical,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
    connection
        .execute_batch("PRAGMA query_only=ON;")
        .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
    let mut statement = connection
        .prepare(
            "SELECT type,name,tbl_name,COALESCE(sql,'')
             FROM sqlite_schema
             WHERE type IN ('table','index') AND name NOT LIKE 'sqlite_%'
             ORDER BY type ASC,name COLLATE NOCASE ASC
             LIMIT ?1",
        )
        .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
    let mut rows = statement
        .query([MAX_DATABASE_SCHEMA_OBJECTS as i64])
        .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
    let mut objects = Vec::new();
    let mut used = 0usize;
    while let Some(row) = rows
        .next()
        .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?
    {
        if cancel.is_cancelled() {
            return Err(DatabasePlatformError::Cancelled);
        }
        let kind: String = row
            .get(0)
            .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
        let name: String = row
            .get(1)
            .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
        let table_name: String = row
            .get(2)
            .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
        let ddl: String = row
            .get(3)
            .map_err(|error| DatabasePlatformError::Unavailable(error.to_string()))?;
        let base = kind.len() + name.len() + table_name.len();
        if used.saturating_add(base) >= MAX_DATABASE_SCHEMA_BYTES {
            break;
        }
        let remaining = MAX_DATABASE_SCHEMA_BYTES - used - base;
        let ddl = truncate_utf8(&normalize_ddl(&ddl), remaining);
        used += base + ddl.len();
        objects.push(DatabaseSchemaObject {
            kind,
            name,
            table_name,
            ddl,
        });
    }
    Ok(objects)
}

fn normalize_ddl(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.into();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].into()
}

fn require_program(path: &str, label: &str) -> Result<String, DatabasePlatformError> {
    let path = Path::new(path);
    if is_executable(path) {
        path_to_string(path)
    } else {
        Err(DatabasePlatformError::NotConfigured(format!(
            "{label} is not executable at {}",
            path.display()
        )))
    }
}

fn find_path_program(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn path_to_string(path: &Path) -> Result<String, DatabasePlatformError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| DatabasePlatformError::Invalid("path is not valid UTF-8".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[tokio::test]
    async fn canonicalizes_regular_sqlite_and_reads_bounded_schema_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("fixture.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT);
                 CREATE INDEX users_name ON users(name);",
            )
            .unwrap();
        drop(connection);
        let platform = MacDatabasePlatform;
        let canonical = platform
            .canonicalize_sqlite(&path, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(canonical, std::fs::canonicalize(&path).unwrap());
        let objects = platform
            .sqlite_schema(&path, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(objects.len(), 2);
        assert!(objects.iter().any(|object| object.name == "users"));
        assert!(objects.iter().all(|object| !object.ddl.contains('\n')));
    }

    #[tokio::test]
    async fn missing_directory_parent_traversal_and_cancel_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let platform = MacDatabasePlatform;
        assert_eq!(
            platform
                .canonicalize_sqlite(
                    &temp.path().join("missing.sqlite"),
                    CancellationToken::new()
                )
                .await
                .unwrap_err(),
            DatabasePlatformError::NotFound
        );
        assert!(matches!(
            platform
                .canonicalize_sqlite(temp.path(), CancellationToken::new())
                .await,
            Err(DatabasePlatformError::Invalid(_))
        ));
        assert!(matches!(
            platform
                .canonicalize_sqlite(
                    &temp.path().join("../escape.sqlite"),
                    CancellationToken::new()
                )
                .await,
            Err(DatabasePlatformError::Invalid(_))
        ));
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert_eq!(
            platform
                .canonicalize_sqlite(temp.path(), cancel)
                .await
                .unwrap_err(),
            DatabasePlatformError::Cancelled
        );
        assert!(matches!(
            require_program("/definitely/missing/luma-database-client", "fixture"),
            Err(DatabasePlatformError::NotConfigured(_))
        ));
    }

    #[test]
    fn postgres_validation_rejects_dsns_and_password_shaped_values() {
        assert!(validate_postgres_metadata("db.example", 5432, "app", "reader").is_ok());
        assert!(validate_postgres_metadata(
            "postgres://reader:secret@db/app",
            5432,
            "app",
            "reader"
        )
        .is_err());
        assert!(validate_postgres_metadata("db.example", 5432, "password=x", "reader").is_err());
    }

    #[test]
    fn client_vectors_are_direct_and_do_not_interpolate_host() {
        let args = postgres_client_args("db.example.test", 5433, "app", "reader");
        assert_eq!(
            args,
            vec![
                "--host",
                "db.example.test",
                "--port",
                "5433",
                "--username",
                "reader",
                "--dbname",
                "app"
            ]
        );
        assert!(!args.iter().any(|arg| arg.contains("password")));
    }
}
