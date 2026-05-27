use async_trait::async_trait;
use emdash_core::{ApiError, DatabaseProvider};
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite, SqlitePool};

// ── Identifier validation ─────────────────────────────────────────────────────

/// Ensures a table or column name contains only alphanumeric characters and
/// underscores.  Prevents SQL injection in dynamic DDL statements.
pub fn validate_identifier(name: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::BadRequest("identifier must not be empty".into()));
    }
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "identifier '{name}' contains invalid characters (only [a-zA-Z0-9_] allowed)"
        )))
    }
}

// ── Column definition ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub required: bool,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub enum ColumnType {
    Text,
    Integer,
    Float,
    Boolean,
    Json,
    Timestamp,
}

impl ColumnType {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ColumnType::Text => "TEXT",
            ColumnType::Integer => "INTEGER",
            ColumnType::Float => "REAL",
            ColumnType::Boolean => "INTEGER",
            ColumnType::Json => "TEXT",
            ColumnType::Timestamp => "TEXT",
        }
    }
}

// ── Migration runner ──────────────────────────────────────────────────────────

const MIGRATIONS: &[(&str, &str)] = &[("001_init", include_str!("../migrations/001_init.sql"))];

async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), ApiError> {
    // Ensure the migrations table exists first.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _emdash_migrations (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    for (name, sql) in MIGRATIONS {
        let already_applied: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _emdash_migrations WHERE name = ?")
                .bind(name)
                .fetch_one(pool)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

        if already_applied == 0 {
            // Run each statement in the migration file individually.
            for statement in sql.split(';') {
                let stmt = statement.trim();
                if !stmt.is_empty() {
                    sqlx::query(stmt)
                        .execute(pool)
                        .await
                        .map_err(|e| ApiError::Internal(format!("migration {name}: {e}")))?;
                }
            }
            sqlx::query("INSERT INTO _emdash_migrations (name) VALUES (?)")
                .bind(name)
                .execute(pool)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            tracing::info!(migration = name, "applied migration");
        }
    }
    Ok(())
}

// ── BespokeDb ─────────────────────────────────────────────────────────────────

/// Connection-pooled SQLite database.  Configured via `DATABASE_URL`
/// environment variable (e.g. `sqlite:./emdash.db`).
pub struct BespokeDb {
    pool: Pool<Sqlite>,
}

impl BespokeDb {
    /// Open (or create) the database and run pending migrations.
    pub async fn connect(database_url: &str) -> Result<Self, ApiError> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| ApiError::Internal(format!("db connect: {e}")))?;

        let db = Self { pool };
        run_migrations(&db.pool).await?;
        Ok(db)
    }

    /// Expose the underlying pool for crates that need direct access.
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Create a collection table dynamically (`ec_<name>`).
    pub async fn create_collection_table(
        &self,
        collection_name: &str,
        columns: &[ColumnDef],
    ) -> Result<(), ApiError> {
        validate_identifier(collection_name)?;
        let table = format!("ec_{collection_name}");

        let mut col_defs = vec![
            "  id         TEXT PRIMARY KEY".to_string(),
            "  status     TEXT NOT NULL DEFAULT 'draft'".to_string(),
            "  slug       TEXT".to_string(),
            "  published_at TEXT".to_string(),
            "  created_at TEXT NOT NULL DEFAULT (datetime('now'))".to_string(),
            "  updated_at TEXT NOT NULL DEFAULT (datetime('now'))".to_string(),
        ];

        for col in columns {
            validate_identifier(&col.name)?;
            let mut def = format!("  {} {}", col.name, col.col_type.to_sql());
            if col.required {
                def.push_str(" NOT NULL");
            }
            if col.unique {
                def.push_str(" UNIQUE");
            }
            col_defs.push(def);
        }

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {table} (\n{}\n)",
            col_defs.join(",\n")
        );

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Fetch a single row by id from any table, returned as a JSON object.
    pub async fn get_by_id(&self, table: &str, id: &str) -> Result<Option<Value>, ApiError> {
        validate_identifier(table)?;
        let sql = format!("SELECT * FROM {table} WHERE id = ?");
        let row_opt = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(row_opt.map(|row| row_to_json(&row)))
    }

    /// List all rows from a table as JSON objects.
    pub async fn list(&self, table: &str) -> Result<Vec<Value>, ApiError> {
        validate_identifier(table)?;
        let sql = format!("SELECT * FROM {table}");
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(rows.iter().map(row_to_json).collect())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_json(row: &sqlx::sqlite::SqliteRow) -> Value {
    use sqlx::Column;
    let mut map = serde_json::Map::new();
    for col in row.columns() {
        let name = col.name();
        // Try each type in order; fall back to null.
        let val: Value = if let Ok(v) = row.try_get::<String, _>(name) {
            // Attempt to parse as JSON first (for JSON columns).
            serde_json::from_str(&v).unwrap_or(Value::String(v))
        } else if let Ok(v) = row.try_get::<i64, _>(name) {
            Value::Number(v.into())
        } else if let Ok(v) = row.try_get::<f64, _>(name) {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        } else if let Ok(v) = row.try_get::<bool, _>(name) {
            Value::Bool(v)
        } else {
            Value::Null
        };
        map.insert(name.to_string(), val);
    }
    Value::Object(map)
}

// ── DatabaseProvider impl ─────────────────────────────────────────────────────

#[async_trait]
impl DatabaseProvider for BespokeDb {
    async fn query(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Value>, ApiError> {
        let mut q = sqlx::query(sql);
        for p in &params {
            q = match p {
                Value::String(s) => q.bind(s.clone()),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        q.bind(i)
                    } else {
                        q.bind(n.as_f64().unwrap_or(0.0))
                    }
                }
                Value::Bool(b) => q.bind(*b),
                Value::Null => q.bind(Option::<String>::None),
                other => q.bind(other.to_string()),
            };
        }
        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(rows.iter().map(row_to_json).collect())
    }

    async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<u64, ApiError> {
        let mut q = sqlx::query(sql);
        for p in &params {
            q = match p {
                Value::String(s) => q.bind(s.clone()),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        q.bind(i)
                    } else {
                        q.bind(n.as_f64().unwrap_or(0.0))
                    }
                }
                Value::Bool(b) => q.bind(*b),
                Value::Null => q.bind(Option::<String>::None),
                other => q.bind(other.to_string()),
            };
        }
        let result = q
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(result.rows_affected())
    }

    async fn get_by_id(&self, table: &str, id: &str) -> Result<Option<Value>, ApiError> {
        self.get_by_id(table, id).await
    }

    async fn list(&self, table: &str) -> Result<Vec<Value>, ApiError> {
        BespokeDb::list(self, table).await
    }
}
