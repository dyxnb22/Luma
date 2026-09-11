use crate::ports::{
    DatabasePortal, DatabasePortalTarget, DatabasePortalsRepoError, DatabasePortalsRepository,
    NewDatabasePortal,
};
use luma_storage::{DatabasePortalRow, DatabasePortalsStore, DatabasePortalsStoreError};
use std::path::PathBuf;
use std::sync::Arc;

pub struct SqliteDatabasePortalsRepository {
    store: Arc<DatabasePortalsStore>,
}

impl SqliteDatabasePortalsRepository {
    pub fn new(store: Arc<DatabasePortalsStore>) -> Self {
        Self { store }
    }
}

impl DatabasePortalsRepository for SqliteDatabasePortalsRepository {
    fn list(&self) -> Result<Vec<DatabasePortal>, DatabasePortalsRepoError> {
        self.store
            .list()?
            .into_iter()
            .map(portal_from_row)
            .collect()
    }

    fn get(&self, id: i64) -> Result<Option<DatabasePortal>, DatabasePortalsRepoError> {
        self.store
            .get(id)
            .map_err(map_error)?
            .map(portal_from_row)
            .transpose()
    }

    fn insert(
        &self,
        portal: &NewDatabasePortal,
    ) -> Result<DatabasePortal, DatabasePortalsRepoError> {
        self.store
            .insert(&row_from_new(portal))
            .map_err(map_error)
            .and_then(portal_from_row)
    }

    fn delete(&self, id: i64, expected_updated_at: &str) -> Result<(), DatabasePortalsRepoError> {
        self.store
            .delete(id, expected_updated_at)
            .map_err(map_error)
    }

    fn backup(&self) -> Result<PathBuf, DatabasePortalsRepoError> {
        self.store.backup().map_err(map_error)
    }
}

fn portal_from_row(row: DatabasePortalRow) -> Result<DatabasePortal, DatabasePortalsRepoError> {
    let target = match row.kind.as_str() {
        "sqlite" => DatabasePortalTarget::Sqlite {
            path: PathBuf::from(
                row.sqlite_path
                    .ok_or_else(|| malformed("sqlite portal is missing path"))?,
            ),
        },
        "postgres" => DatabasePortalTarget::Postgres {
            host: row
                .pg_host
                .ok_or_else(|| malformed("postgres portal is missing host"))?,
            port: row
                .pg_port
                .ok_or_else(|| malformed("postgres portal is missing port"))?,
            database: row
                .pg_database
                .ok_or_else(|| malformed("postgres portal is missing database"))?,
            username: row
                .pg_username
                .ok_or_else(|| malformed("postgres portal is missing username"))?,
        },
        _ => return Err(malformed("portal kind is invalid")),
    };
    Ok(DatabasePortal {
        id: row.id,
        label: row.label,
        target,
        environment: row.environment,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn row_from_new(portal: &NewDatabasePortal) -> DatabasePortalRow {
    let (kind, sqlite_path, pg_host, pg_port, pg_database, pg_username) = match &portal.target {
        DatabasePortalTarget::Sqlite { path } => (
            "sqlite".into(),
            Some(path.display().to_string()),
            None,
            None,
            None,
            None,
        ),
        DatabasePortalTarget::Postgres {
            host,
            port,
            database,
            username,
        } => (
            "postgres".into(),
            None,
            Some(host.clone()),
            Some(*port),
            Some(database.clone()),
            Some(username.clone()),
        ),
    };
    DatabasePortalRow {
        id: 0,
        label: portal.label.clone(),
        kind,
        sqlite_path,
        pg_host,
        pg_port,
        pg_database,
        pg_username,
        environment: portal.environment.clone(),
        created_at: portal.now.clone(),
        updated_at: portal.now.clone(),
    }
}

fn malformed(message: &str) -> DatabasePortalsRepoError {
    DatabasePortalsRepoError::Store(format!("malformed metadata: {message}"))
}

fn map_error(error: DatabasePortalsStoreError) -> DatabasePortalsRepoError {
    match error {
        DatabasePortalsStoreError::NotFound => DatabasePortalsRepoError::NotFound,
        DatabasePortalsStoreError::Conflict => DatabasePortalsRepoError::Conflict,
        DatabasePortalsStoreError::Duplicate => DatabasePortalsRepoError::Duplicate,
        DatabasePortalsStoreError::Capacity => DatabasePortalsRepoError::Capacity,
        other => DatabasePortalsRepoError::Store(other.to_string()),
    }
}

impl From<DatabasePortalsStoreError> for DatabasePortalsRepoError {
    fn from(value: DatabasePortalsStoreError) -> Self {
        map_error(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_round_trip_preserves_non_secret_postgres_fields() {
        let temp = tempfile::tempdir().unwrap();
        let repository = SqliteDatabasePortalsRepository::new(Arc::new(
            DatabasePortalsStore::with_path(temp.path().join("database_portals.sqlite")).unwrap(),
        ));
        let portal = repository
            .insert(&NewDatabasePortal {
                label: "Staging".into(),
                target: DatabasePortalTarget::Postgres {
                    host: "db.example.test".into(),
                    port: 5432,
                    database: "app".into(),
                    username: "reader".into(),
                },
                environment: "staging".into(),
                now: "v1".into(),
            })
            .unwrap();
        assert_eq!(repository.get(portal.id).unwrap(), Some(portal));
    }
}
