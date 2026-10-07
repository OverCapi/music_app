//! Accounts and session tokens in SQLite. Tokens are stored as SHA-256 hashes so a
//! leaked database does not leak usable sessions.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use rusqlite::{Connection, ErrorCode, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};
use sha2::{Digest, Sha256};

/// Schema versions, tracked in SQLite's `user_version`. Append only.
const MIGRATION_LIST: &[M<'_>] = &[M::up(include_str!("migrations/01_init.sql"))];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATION_LIST);

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("username taken")]
    UsernameTaken,
    #[error("bad credentials")]
    BadCredentials,
    #[error("database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("password hash: {0}")]
    PasswordHash(#[from] argon2::password_hash::Error),
    #[error("OS random generator: {0}")]
    Random(#[from] getrandom::Error),
    #[error("blocking task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Cheap to clone; every call runs on the blocking thread pool.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Opens (or creates) the database and migrates it to the latest schema.
    pub fn open(path: &Path) -> Result<Self, rusqlite_migration::Error> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        MIGRATIONS.to_latest(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Creates the account and returns a first session token.
    pub async fn register(&self, username: String, password: String) -> Result<String, AuthError> {
        let password_hash = blocking(move || hash_password(&password)).await??;
        let db = self.clone();
        blocking(move || {
            let conn = db.lock();
            let inserted = conn.execute(
                "INSERT INTO users (username, password_hash, created_at) VALUES (?1, ?2, ?3)",
                params![username, password_hash, now_secs()],
            );
            match inserted {
                Err(rusqlite::Error::SqliteFailure(e, _))
                    if e.code == ErrorCode::ConstraintViolation =>
                {
                    Err(AuthError::UsernameTaken)
                }
                Err(e) => Err(e.into()),
                Ok(_) => issue_token(&conn, conn.last_insert_rowid()),
            }
        })
        .await?
    }

    /// Checks the password and returns `(canonical username, new token)`.
    pub async fn login(
        &self,
        username: String,
        password: String,
    ) -> Result<(String, String), AuthError> {
        let db = self.clone();
        let user = blocking(move || {
            db.lock()
                .query_row(
                    "SELECT id, username, password_hash FROM users WHERE username = ?1",
                    params![username],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
        })
        .await??;
        let Some((user_id, username, stored)) = user else {
            return Err(AuthError::BadCredentials);
        };
        if !blocking(move || verify_password(&password, &stored)).await? {
            return Err(AuthError::BadCredentials);
        }
        let db = self.clone();
        let token = blocking(move || issue_token(&db.lock(), user_id)).await??;
        Ok((username, token))
    }

    /// Resolves a session token to its username.
    pub async fn user_for_token(&self, token: String) -> Result<Option<String>, AuthError> {
        let db = self.clone();
        Ok(blocking(move || {
            db.lock()
                .query_row(
                    "SELECT u.username FROM tokens t JOIN users u ON u.id = t.user_id
                     WHERE t.token_hash = ?1",
                    params![hash_token(&token)],
                    |r| r.get(0),
                )
                .optional()
        })
        .await??)
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        // A panic mid-query cannot leave the connection itself inconsistent.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AuthError> {
    Ok(tokio::task::spawn_blocking(f).await?)
}

fn issue_token(conn: &Connection, user_id: i64) -> Result<String, AuthError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    let token = hex::encode(bytes);
    conn.execute(
        "INSERT INTO tokens (token_hash, user_id, created_at) VALUES (?1, ?2, ?3)",
        params![hash_token(&token), user_id, now_secs()],
    )?;
    Ok(token)
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// PHC string with a random salt.
fn hash_password(password: &str) -> Result<String, AuthError> {
    Ok(Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string())
}

fn verify_password(password: &str, stored: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), stored)
        .is_ok()
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        MIGRATIONS.validate().unwrap();
    }

    #[test]
    fn passwords() {
        let hash = hash_password("secret123").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("secret123", &hash));
        assert!(!verify_password("secret124", &hash));
        assert!(!verify_password("secret123", "not a phc string"));
    }
}
